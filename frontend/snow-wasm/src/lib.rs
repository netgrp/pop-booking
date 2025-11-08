use std::array;
use std::cell::RefCell;
use std::f32::consts::PI;
use std::thread_local;

const TARGET_FPS: f32 = 60.0;
const FIXED_TIMESTEP: f32 = 1000.0 / TARGET_FPS;
const MAX_STEPS_PER_FRAME: usize = 4;
const MAX_PARTICLES: usize = 10_000;
const MAX_ACTIVE_PARTICLES: usize = MAX_PARTICLES;
const ACTIVE_RENDER_STRIDE: usize = 3;
const STATIC_RENDER_STRIDE: usize = 3;
const GRID_COLS: usize = 32;
const GRID_ROWS: usize = 22;
const GRID_CELL_RESERVE: usize = 32;
const NEIGHBOR_RESERVE: usize = 256;
const INITIAL_SPAWN: usize = 250;
const MAX_SPAWN_PER_STEP: usize = 48;
const SPAWN_RATE_PER_SECOND: f32 = 220.0;
const SPAWN_RATE_PER_MS: f32 = SPAWN_RATE_PER_SECOND / 1000.0;
const RELAXATION_STEPS: usize = 1;
const GRAVITY_SCALE: f32 = 0.0035;
const TERMINAL_VELOCITY: f32 = 1.9 * 1.35;
const MIN_CELL_SIZE: f32 = 8.0;
const GRID_CELL_COUNT: usize = GRID_COLS * GRID_ROWS;
const ACTIVE_RENDER_CAP: usize = MAX_ACTIVE_PARTICLES * ACTIVE_RENDER_STRIDE;
const STATIC_RENDER_CAP: usize = MAX_PARTICLES * STATIC_RENDER_STRIDE;

fn clamp_x(width: f32, radius: f32, x: f32) -> f32 {
    let max_x = (width - radius).max(radius);
    x.clamp(radius, max_x)
}

fn clamp_y(height: f32, radius: f32, y: f32) -> f32 {
    if y + radius > height {
        height - radius
    } else {
        y
    }
}

fn wrap_x(mut x: f32, width: f32) -> f32 {
    if x > width + 5.0 {
        x = -5.0;
    } else if x < -5.0 {
        x = width + 5.0;
    }
    x
}

thread_local! {
    static SIMULATION: RefCell<Option<SnowSimulation>> = RefCell::new(None);
}

#[derive(Clone, Copy)]
struct Particle {
    x: f32,
    y: f32,
    radius: f32,
    vx: f32,
    vy: f32,
    sway: f32,
    angle: f32,
    angle_speed: f32,
}

#[derive(Clone, Copy)]
struct StaticParticle {
    x: f32,
    y: f32,
    radius: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OccupantKind {
    Active,
    Static,
}

#[derive(Clone, Copy)]
struct CellOccupant {
    kind: OccupantKind,
    index: u32,
}

const ZERO_PARTICLE: Particle = Particle {
    x: 0.0,
    y: 0.0,
    radius: 0.0,
    vx: 0.0,
    vy: 0.0,
    sway: 0.0,
    angle: 0.0,
    angle_speed: 0.0,
};

const ZERO_STATIC: StaticParticle = StaticParticle {
    x: 0.0,
    y: 0.0,
    radius: 0.0,
};

const EMPTY_OCCUPANT: CellOccupant = CellOccupant {
    kind: OccupantKind::Static,
    index: 0,
};

struct ParticlePool {
    data: [Particle; MAX_PARTICLES],
    len: usize,
}

impl ParticlePool {
    fn new() -> Self {
        Self {
            data: [ZERO_PARTICLE; MAX_PARTICLES],
            len: 0,
        }
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    fn is_full(&self) -> bool {
        self.len >= MAX_PARTICLES
    }

    #[inline(always)]
    fn clear(&mut self) {
        self.len = 0;
    }

    #[inline(always)]
    fn push(&mut self, particle: Particle) -> bool {
        if self.is_full() {
            return false;
        }
        self.data[self.len] = particle;
        self.len += 1;
        true
    }

    #[inline(always)]
    fn swap_remove(&mut self, index: usize) -> Option<Particle> {
        if index >= self.len {
            return None;
        }
        self.len -= 1;
        let removed = self.data[index];
        self.data[index] = self.data[self.len];
        Some(removed)
    }

    #[inline(always)]
    fn as_slice(&self) -> &[Particle] {
        &self.data[..self.len]
    }

    #[inline(always)]
    fn as_mut_slice(&mut self) -> &mut [Particle] {
        let len = self.len;
        &mut self.data[..len]
    }

    #[inline(always)]
    fn get(&self, index: usize) -> &Particle {
        debug_assert!(index < self.len);
        &self.data[index]
    }

}

struct StaticPool {
    data: [StaticParticle; MAX_PARTICLES],
    len: usize,
}

impl StaticPool {
    fn new() -> Self {
        Self {
            data: [ZERO_STATIC; MAX_PARTICLES],
            len: 0,
        }
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    fn clear(&mut self) {
        self.len = 0;
    }

    #[inline(always)]
    fn push(&mut self, particle: StaticParticle) -> bool {
        if self.len >= MAX_PARTICLES {
            return false;
        }
        self.data[self.len] = particle;
        self.len += 1;
        true
    }

    #[inline(always)]
    fn as_slice(&self) -> &[StaticParticle] {
        &self.data[..self.len]
    }

    #[inline(always)]
    fn as_mut_slice(&mut self) -> &mut [StaticParticle] {
        let len = self.len;
        &mut self.data[..len]
    }

}

struct CellList {
    occupants: [CellOccupant; GRID_CELL_RESERVE],
    len: usize,
}

impl CellList {
    fn new() -> Self {
        Self {
            occupants: [EMPTY_OCCUPANT; GRID_CELL_RESERVE],
            len: 0,
        }
    }

    #[inline(always)]
    fn clear(&mut self) {
        self.len = 0;
    }

    #[inline(always)]
    fn push(&mut self, occupant: CellOccupant) {
        if self.len < GRID_CELL_RESERVE {
            self.occupants[self.len] = occupant;
            self.len += 1;
        }
    }

    #[inline(always)]
    fn iter(&self) -> &[CellOccupant] {
        &self.occupants[..self.len]
    }
}

struct NeighborBuffer {
    data: [CellOccupant; NEIGHBOR_RESERVE],
    len: usize,
}

impl NeighborBuffer {
    fn new() -> Self {
        Self {
            data: [EMPTY_OCCUPANT; NEIGHBOR_RESERVE],
            len: 0,
        }
    }

    #[inline(always)]
    fn clear(&mut self) {
        self.len = 0;
    }

    #[inline(always)]
    fn extend(&mut self, occupants: &[CellOccupant]) {
        for &occupant in occupants {
            if self.len >= NEIGHBOR_RESERVE {
                break;
            }
            self.data[self.len] = occupant;
            self.len += 1;
        }
    }

    #[inline(always)]
    fn iter(&self) -> &[CellOccupant] {
        &self.data[..self.len]
    }
}

struct IndexBuffer<const N: usize> {
    data: [usize; N],
    len: usize,
}

impl<const N: usize> IndexBuffer<N> {
    const fn new() -> Self {
        Self {
            data: [0; N],
            len: 0,
        }
    }

    #[inline(always)]
    fn clear(&mut self) {
        self.len = 0;
    }

    #[inline(always)]
    fn push(&mut self, value: usize) {
        if self.len < N {
            self.data[self.len] = value;
            self.len += 1;
        }
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn sort_and_dedup(&mut self) {
        self.data[..self.len].sort_unstable();
        let mut unique = 0;
        for i in 1..self.len {
            if self.data[i] != self.data[unique] {
                unique += 1;
                self.data[unique] = self.data[i];
            }
        }
        if self.len > 0 {
            self.len = unique + 1;
        }
    }

    #[inline(always)]
    fn pop(&mut self) -> Option<usize> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            Some(self.data[self.len])
        }
    }
}

struct SnowSimulation {
    width: f32,
    height: f32,
    pointer_target: f32,
    pointer_current: f32,
    rng_state: u32,
    active: ParticlePool,
    frozen: StaticPool,
    render_active: [f32; ACTIVE_RENDER_CAP],
    render_static: [f32; STATIC_RENDER_CAP],
    active_render_len: usize,
    static_render_len: usize,
    grid_cells: [CellList; GRID_CELL_COUNT],
    grid_cell_width: f32,
    grid_cell_height: f32,
    grid_inv_cell_width: f32,
    grid_inv_cell_height: f32,
    neighbor_buffer: NeighborBuffer,
    spawn_accumulator: f32,
    freeze_queue: IndexBuffer<MAX_PARTICLES>,
}

impl SnowSimulation {
    fn new(width: f32, height: f32) -> Self {
        let grid_cells = array::from_fn(|_| CellList::new());
        let mut sim = SnowSimulation {
            width,
            height,
            pointer_target: 0.0,
            pointer_current: 0.0,
            rng_state: 0x5EED5EED,
            active: ParticlePool::new(),
            frozen: StaticPool::new(),
            render_active: [0.0; ACTIVE_RENDER_CAP],
            render_static: [0.0; STATIC_RENDER_CAP],
            active_render_len: 0,
            static_render_len: 0,
            grid_cells,
            grid_cell_width: MIN_CELL_SIZE,
            grid_cell_height: MIN_CELL_SIZE,
            grid_inv_cell_width: 0.0,
            grid_inv_cell_height: 0.0,
            neighbor_buffer: NeighborBuffer::new(),
            spawn_accumulator: 0.0,
            freeze_queue: IndexBuffer::new(),
        };
        sim.update_grid_metrics();
        sim.populate_particles();
        sim.update_render_buffers();
        sim
    }

    fn populate_particles(&mut self) {
        self.active.clear();
        self.frozen.clear();
        self.spawn_accumulator = 0.0;
        for _ in 0..INITIAL_SPAWN.min(MAX_PARTICLES) {
            let particle = self.create_particle(None);
            if !self.active.push(particle) {
                break;
            }
        }
    }

    fn create_particle(&mut self, initial_y: Option<f32>) -> Particle {
        let radius = self.rand_range(1.2, 3.2);
        Particle {
            x: self.rand() * self.width.max(1.0),
            y: initial_y.unwrap_or_else(|| self.rand() * self.height.max(1.0)),
            radius,
            vx: 0.0,
            vy: self.rand_range(0.25, 0.95),
            sway: self.rand_range(0.3, 0.95),
            angle: self.rand() * 2.0 * PI,
            angle_speed: self.rand_range(0.007, 0.037),
        }
    }

    fn rand(&mut self) -> f32 {
        self.rng_state = self
            .rng_state
            .wrapping_mul(1664525)
            .wrapping_add(1013904223);
        let value = (self.rng_state >> 8) as f32;
        value / ((u32::MAX >> 8) as f32)
    }

    fn rand_range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.rand()
    }

    fn clear_grid(&mut self) {
        for cell in &mut self.grid_cells {
            cell.clear();
        }
    }

    fn update_grid_metrics(&mut self) {
        self.grid_cell_width = (self.width / GRID_COLS as f32).max(MIN_CELL_SIZE);
        self.grid_cell_height = (self.height / GRID_ROWS as f32).max(MIN_CELL_SIZE);
        self.grid_inv_cell_width = if self.grid_cell_width > 0.0 {
            1.0 / self.grid_cell_width
        } else {
            0.0
        };
        self.grid_inv_cell_height = if self.grid_cell_height > 0.0 {
            1.0 / self.grid_cell_height
        } else {
            0.0
        };
    }

    fn push_to_grid(&mut self, kind: OccupantKind, index: usize, x: f32, y: f32) {
        if self.grid_cell_width <= 0.0 || self.grid_cell_height <= 0.0 {
            return;
        }
        let col =
            ((x * self.grid_inv_cell_width) as isize).clamp(0, (GRID_COLS as isize) - 1) as usize;
        let row = ((y * self.grid_inv_cell_height) as isize)
            .clamp(0, (GRID_ROWS as isize) - 1) as usize;
        let cell_index = row * GRID_COLS + col;
        debug_assert!(cell_index < GRID_CELL_COUNT);
        debug_assert!(index <= u32::MAX as usize);
        self.grid_cells[cell_index].push(CellOccupant {
            kind,
            index: index as u32,
        });
    }

    fn rebuild_grid(&mut self, include_active: bool) {
        self.clear_grid();
        let frozen_len = self.frozen.len();
        for index in 0..frozen_len {
            let particle = self.frozen.as_slice()[index];
            self.push_to_grid(OccupantKind::Static, index, particle.x, particle.y);
        }
        if include_active {
            let active_len = self.active.len();
            for index in 0..active_len {
                let particle = self.active.get(index);
                self.push_to_grid(OccupantKind::Active, index, particle.x, particle.y);
            }
        }
    }

    fn collect_neighbors(&mut self, x: f32, y: f32, radius: f32) {
        self.neighbor_buffer.clear();
        if self.grid_cell_width <= 0.0 || self.grid_cell_height <= 0.0 {
            return;
        }
        let reach_x = radius * 2.5;
        let reach_y = radius * 2.5;
        let min_col = ((x - reach_x) * self.grid_inv_cell_width)
            .floor()
            .max(0.0) as usize;
        let max_col = ((x + reach_x) * self.grid_inv_cell_width)
            .ceil()
            .min((GRID_COLS - 1) as f32) as usize;
        let min_row = ((y - reach_y) * self.grid_inv_cell_height)
            .floor()
            .max(0.0) as usize;
        let max_row = ((y + reach_y) * self.grid_inv_cell_height)
            .ceil()
            .min((GRID_ROWS - 1) as f32) as usize;
        for row in min_row..=max_row {
            for col in min_col..=max_col {
                let idx = row * GRID_COLS + col;
                let cell = &self.grid_cells[idx];
                self.neighbor_buffer.extend(cell.iter());
            }
        }
    }

    fn freeze_active(&mut self, index: usize) {
        if index >= self.active.len() {
            return;
        }
        if let Some(particle) = self.active.swap_remove(index) {
            let radius = particle.radius;
            let clamped_x = clamp_x(self.width, radius, particle.x);
            let clamped_y = clamp_y(self.height, radius, particle.y);
            let _ = self.frozen.push(StaticParticle {
                x: clamped_x,
                y: clamped_y,
                radius,
            });
        }
    }

    fn apply_freezes(&mut self) {
        if self.freeze_queue.is_empty() {
            return;
        }
        self.freeze_queue.sort_and_dedup();
        while let Some(index) = self.freeze_queue.pop() {
            self.freeze_active(index);
        }
    }

    fn update_active_particles(&mut self, wind: f32, delta_ratio: f32) {
        for particle in self.active.as_mut_slice().iter_mut() {
            particle.angle += particle.angle_speed * delta_ratio;
            let sway = particle.angle.sin() * particle.sway;
            particle.vx += (sway + wind - particle.vx) * 0.08 * delta_ratio;
            particle.vy = (particle.vy
                + GRAVITY_SCALE * (1.0 + particle.radius * 0.35) * delta_ratio)
                .min(TERMINAL_VELOCITY);
            particle.x += particle.vx * delta_ratio;
            particle.y += particle.vy * 1.8 * delta_ratio;
            particle.x = wrap_x(particle.x, self.width);
        }
    }

    fn spawn_new_particles(&mut self, delta_ms: f32) {
        if self.active.len() + self.frozen.len() >= MAX_PARTICLES {
            return;
        }
        self.spawn_accumulator += delta_ms * SPAWN_RATE_PER_MS;
        let spawn_ready = self.spawn_accumulator.floor() as usize;
        if spawn_ready == 0 {
            return;
        }
        let available = MAX_PARTICLES - (self.active.len() + self.frozen.len());
        let to_spawn = spawn_ready.min(available).min(MAX_SPAWN_PER_STEP);
        for _ in 0..to_spawn {
            let particle = self.create_particle(Some(-20.0));
            if !self.active.push(particle) {
                break;
            }
        }
        self.spawn_accumulator -= to_spawn as f32;
    }

    fn resolve_particle_collisions(&mut self, index: usize) -> bool {
        let active_len = self.active.len();
        if index >= active_len {
            return false;
        }
        let (px, py, pr) = {
            let particle = self.active.get(index);
            (particle.x, particle.y, particle.radius)
        };
        self.collect_neighbors(px, py, pr);
        let slice = self.active.as_mut_slice();
        let (left, rest) = slice.split_at_mut(index);
        let (particle, right) = rest.split_first_mut().unwrap();
        let mut support_contacts = 0;
        for &entry in self.neighbor_buffer.iter() {
            let entry_index = entry.index as usize;
            if entry.kind == OccupantKind::Active && entry_index == index {
                continue;
            }
            match entry.kind {
                OccupantKind::Active => {
                    if entry_index >= active_len {
                        continue;
                    }
                    let neighbor = if entry_index < index {
                        &mut left[entry_index]
                    } else {
                        &mut right[entry_index - index - 1]
                    };
                    resolve_pair(particle, neighbor, &mut support_contacts);
                }
                OccupantKind::Static => {
                    if entry_index >= self.frozen.len() {
                        continue;
                    }
                    let neighbor = self.frozen.as_slice()[entry_index];
                    resolve_with_static(particle, neighbor, &mut support_contacts);
                }
            }
        }

        if particle.y + particle.radius >= self.height - 1.0 {
            particle.y = self.height - particle.radius;
            support_contacts += 2;
        }

        particle.x = wrap_x(particle.x, self.width);
        support_contacts >= 2
    }

    fn run_collision_relaxation(&mut self) {
        if self.active.is_empty() {
            return;
        }
        self.rebuild_grid(true);
        for step in 0..RELAXATION_STEPS {
            self.freeze_queue.clear();
            let active_len = self.active.len();
            for index in 0..active_len {
                if self.resolve_particle_collisions(index) {
                    self.freeze_queue.push(index);
                }
            }
            if !self.freeze_queue.is_empty() {
                self.apply_freezes();
            }
            if step + 1 < RELAXATION_STEPS && !self.active.is_empty() {
                self.rebuild_grid(true);
            }
        }
    }

    fn update_render_buffers(&mut self) {
        let max_active = ACTIVE_RENDER_CAP / ACTIVE_RENDER_STRIDE;
        let active_slice = self.active.as_slice();
        let active_count = active_slice.len().min(max_active);
        self.active_render_len = active_count * ACTIVE_RENDER_STRIDE;
        for (i, particle) in active_slice.iter().take(active_count).enumerate() {
            let base = i * ACTIVE_RENDER_STRIDE;
            self.render_active[base] = particle.x;
            self.render_active[base + 1] = particle.y;
            self.render_active[base + 2] = particle.radius;
        }

        let max_static = STATIC_RENDER_CAP / STATIC_RENDER_STRIDE;
        let frozen_slice = self.frozen.as_slice();
        let static_total = frozen_slice.len().min(max_static);
        self.static_render_len = static_total * STATIC_RENDER_STRIDE;
        let mut offset = 0;
        for particle in frozen_slice.iter().take(static_total) {
            if offset >= self.static_render_len {
                break;
            }
            self.render_static[offset] = particle.x;
            self.render_static[offset + 1] = particle.y;
            self.render_static[offset + 2] = particle.radius;
            offset += STATIC_RENDER_STRIDE;
        }
    }

    fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.update_grid_metrics();
        for particle in self.active.as_mut_slice().iter_mut() {
            particle.x = particle.x.clamp(0.0, self.width);
            particle.y = particle.y.clamp(0.0, self.height);
        }
        for particle in self.frozen.as_mut_slice().iter_mut() {
            particle.x = clamp_x(self.width, particle.radius, particle.x);
            particle.y = clamp_y(self.height, particle.radius, particle.y);
        }
        self.update_render_buffers();
    }

    fn step(&mut self, delta_ms: f32) {
        if delta_ms <= 0.0 {
            return;
        }
        let clamped_delta = delta_ms.min(1000.0);
        let delta_ratio = (clamped_delta / FIXED_TIMESTEP).clamp(0.1, 3.5);
        self.pointer_current += (self.pointer_target - self.pointer_current) * 0.02 * delta_ratio;
        let wind = self.pointer_current * 2.2;
        self.spawn_new_particles(clamped_delta);
        self.update_active_particles(wind, delta_ratio);
        self.run_collision_relaxation();
        self.update_render_buffers();
    }

    fn set_pointer_target(&mut self, target: f32) {
        self.pointer_target = target;
    }

    fn reset(&mut self) {
        self.pointer_current = 0.0;
        self.pointer_target = 0.0;
        self.frozen.clear();
        self.populate_particles();
        self.update_render_buffers();
    }

    fn active_ptr(&self) -> *const f32 {
        self.render_active.as_ptr()
    }

    fn static_ptr(&self) -> *const f32 {
        self.render_static.as_ptr()
    }

    fn active_len(&self) -> u32 {
        self.active_render_len as u32
    }

    fn static_len(&self) -> u32 {
        self.static_render_len as u32
    }

    fn dynamic_count(&self) -> u32 {
        self.active.len() as u32
    }

    fn inactive_count(&self) -> u32 {
        self.frozen.len() as u32
    }
}

fn resolve_pair(particle: &mut Particle, neighbor: &mut Particle, support_contacts: &mut i32) {
    let dx = particle.x - neighbor.x;
    let dy = particle.y - neighbor.y;
    if dy <= 0.0 {
        return;
    }
    let distance = (dx * dx + dy * dy).sqrt().max(0.0001);
    let min_dist = particle.radius + neighbor.radius + 0.25;
    if distance >= min_dist {
        return;
    }
    let overlap = min_dist - distance;
    let nx = dx / distance;
    let ny = dy / distance;
    let compression = overlap * 0.65 + 0.02;
    particle.x += nx * compression;
    particle.y += ny * compression;
    particle.vx += nx * compression * 0.04;
    particle.vy += ny * compression * 0.04;
    neighbor.x -= nx * compression;
    neighbor.y -= ny * compression;
    neighbor.vx -= nx * compression * 0.04;
    neighbor.vy -= ny * compression * 0.04;
    if ny > 0.35 {
        *support_contacts += 1;
    }
}

fn resolve_with_static(
    particle: &mut Particle,
    neighbor: StaticParticle,
    support_contacts: &mut i32,
) {
    let dx = particle.x - neighbor.x;
    let dy = particle.y - neighbor.y;
    if dy <= 0.0 {
        return;
    }
    let distance = (dx * dx + dy * dy).sqrt().max(0.0001);
    let min_dist = particle.radius + neighbor.radius + 0.25;
    if distance >= min_dist {
        return;
    }
    let overlap = min_dist - distance;
    let nx = dx / distance;
    let ny = dy / distance;
    let compression = overlap * 0.65 + 0.02;
    particle.x += nx * compression;
    particle.y += ny * compression;
    particle.vx += nx * compression * 0.04;
    particle.vy += ny * compression * 0.04;
    if ny > 0.35 {
        *support_contacts += 1;
    }
}

fn with_simulation<F, R>(default: R, mut f: F) -> R
where
    F: FnMut(&mut SnowSimulation) -> R,
{
    SIMULATION.with(|cell| {
        let mut borrow = cell.borrow_mut();
        if let Some(sim) = borrow.as_mut() {
            f(sim)
        } else {
            default
        }
    })
}

#[no_mangle]
pub extern "C" fn snow_init(width: f32, height: f32) {
    SIMULATION.with(|cell| {
        *cell.borrow_mut() = Some(SnowSimulation::new(width.max(1.0), height.max(1.0)));
    });
}

#[no_mangle]
pub extern "C" fn snow_resize(width: f32, height: f32) {
    with_simulation((), |sim| sim.resize(width.max(1.0), height.max(1.0)));
}

#[no_mangle]
pub extern "C" fn snow_pointer_wind(target: f32) {
    with_simulation((), |sim| sim.set_pointer_target(target));
}

#[no_mangle]
pub extern "C" fn snow_step(delta_ms: f32) {
    with_simulation((), |sim| {
        let clamped = delta_ms.clamp(0.1, 100.0);
        let mut remaining = clamped;
        for _ in 0..MAX_STEPS_PER_FRAME {
            let step_delta = remaining.min(FIXED_TIMESTEP);
            sim.step(step_delta);
            if remaining <= FIXED_TIMESTEP {
                break;
            }
            remaining -= FIXED_TIMESTEP;
        }
    });
}

#[no_mangle]
pub extern "C" fn snow_active_ptr() -> *const f32 {
    with_simulation(std::ptr::null(), |sim| sim.active_ptr())
}

#[no_mangle]
pub extern "C" fn snow_active_len() -> u32 {
    with_simulation(0, |sim| sim.active_len())
}

#[no_mangle]
pub extern "C" fn snow_static_ptr() -> *const f32 {
    with_simulation(std::ptr::null(), |sim| sim.static_ptr())
}

#[no_mangle]
pub extern "C" fn snow_static_len() -> u32 {
    with_simulation(0, |sim| sim.static_len())
}

#[no_mangle]
pub extern "C" fn snow_dynamic_count() -> u32 {
    with_simulation(0, |sim| sim.dynamic_count())
}

#[no_mangle]
pub extern "C" fn snow_inactive_count() -> u32 {
    with_simulation(0, |sim| sim.inactive_count())
}

#[no_mangle]
pub extern "C" fn snow_reset() {
    with_simulation((), |sim| sim.reset());
}
