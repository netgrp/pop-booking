mod shaders;

use js_sys::Float32Array;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext, WebGlBuffer, WebGlProgram, WebGlShader,
    WebGlTexture, WebGlUniformLocation, WebGlVertexArrayObject,
};

const TARGET_FPS: f32 = 60.0;
const FIXED_TIMESTEP: f32 = 1000.0 / TARGET_FPS;
const MAX_PARTICLES: usize = 10_000;
const TEXTURE_WIDTH: usize = 128;
const TEXTURE_HEIGHT: usize = 128;
const PARTICLE_CAPACITY: usize = TEXTURE_WIDTH * TEXTURE_HEIGHT;
const TEXEL_STRIDE: usize = 4;
const TEXTURE_FLOATS: usize = PARTICLE_CAPACITY * TEXEL_STRIDE;
const PILE_BINS: usize = 256;
const FLOW_WIDTH: usize = 64;
const FLOW_HEIGHT: usize = 64;

thread_local! {
    static RENDERER: RefCell<Option<SnowRenderer>> = RefCell::new(None);
}

#[wasm_bindgen(start)]
pub fn wasm_start() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn snow_init(width: f32, height: f32) -> Result<(), JsValue> {
    RENDERER.with(|cell| {
        let renderer = SnowRenderer::new(width.max(1.0), height.max(1.0))?;
        *cell.borrow_mut() = Some(renderer);
        Ok(())
    })
}

#[wasm_bindgen]
pub fn snow_resize(width: f32, height: f32) -> Result<(), JsValue> {
    with_renderer(|renderer| renderer.resize(width.max(1.0), height.max(1.0)))
}

#[wasm_bindgen]
pub fn snow_pointer_wind(target: f32) {
    let _ = with_renderer(|renderer| {
        renderer.pointer_target = target;
        Ok(())
    });
}

#[wasm_bindgen]
pub fn snow_pointer_move(x: f32, y: f32, dx: f32, dy: f32) -> Result<(), JsValue> {
    with_renderer(|renderer| {
        renderer.pointer_pos = [x, y];
        renderer.pointer_delta = [dx, dy];
        Ok(())
    })
}

#[wasm_bindgen]
pub fn snow_step(delta_ms: f32) -> Result<(), JsValue> {
    with_renderer(|renderer| renderer.step(delta_ms))
}

#[wasm_bindgen]
pub fn snow_reset() -> Result<(), JsValue> {
    with_renderer(|renderer| renderer.reset())
}

#[wasm_bindgen]
pub fn snow_set_tint(r: f32, g: f32, b: f32) -> Result<(), JsValue> {
    with_renderer(|renderer| {
        renderer.set_tint([r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)]);
        Ok(())
    })
}

#[wasm_bindgen]
pub fn snow_dynamic_count() -> u32 {
    MAX_PARTICLES as u32
}

#[wasm_bindgen]
pub fn snow_inactive_count() -> u32 {
    0
}

#[wasm_bindgen]
pub fn snow_pile_bins() -> Result<Float32Array, JsValue> {
    with_renderer(|renderer| Ok(Float32Array::from(renderer.pile_smoothed.as_slice())))
}

fn with_renderer<F, R>(f: F) -> Result<R, JsValue>
where
    F: FnOnce(&mut SnowRenderer) -> Result<R, JsValue>,
{
    RENDERER.with(|cell| {
        let mut borrow = cell.borrow_mut();
        match borrow.as_mut() {
            Some(renderer) => f(renderer),
            None => Err(js_err("snow renderer is not initialized")),
        }
    })
}

fn js_err(message: &str) -> JsValue {
    JsValue::from_str(message)
}

struct SnowRenderer {
    canvas: HtmlCanvasElement,
    gl: WebGl2RenderingContext,
    render_program: WebGlProgram,
    render_vao: WebGlVertexArrayObject,
    _particle_buffer: WebGlBuffer,
    texture: WebGlTexture,
    width: f32,
    height: f32,
    pointer_target: f32,
    pointer_current: f32,
    pointer_pos: [f32; 2],
    pointer_delta: [f32; 2],
    time_ms: f32,
    rng_state: u32,
    tint: [f32; 3],
    texture_size: [f32; 2],
    particle_velocity: Vec<[f32; 2]>,
    particle_data: Vec<f32>,
    pile_bins: Vec<f32>,
    pile_smoothed: Vec<f32>,
    flow_field: FlowField,
    spawn_accum_ms: f32,
    next_spawn_ms: f32,
    spawn_cursor: usize,
    render_uniforms: RenderUniforms,
}

#[derive(Default)]
struct RenderUniforms {
    particles: Option<WebGlUniformLocation>,
    viewport: Option<WebGlUniformLocation>,
    texture_size: Option<WebGlUniformLocation>,
    point_scale: Option<WebGlUniformLocation>,
    tint: Option<WebGlUniformLocation>,
}

struct FlowField {
    width: usize,
    height: usize,
    data: Vec<[f32; 2]>,
}

impl FlowField {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![[0.0; 2]; width * height],
        }
    }

    fn clear(&mut self) {
        for v in &mut self.data {
            *v = [0.0, 0.0];
        }
    }

    fn update(&mut self, time: f32, viewport: [f32; 2], cursor_pos: [f32; 2], cursor_delta: [f32; 2]) {
        let t = time * 0.25;
        let freq = 3.2;
        for y in 0..self.height {
            for x in 0..self.width {
                let nx = x as f32 * freq / self.width as f32;
                let ny = y as f32 * freq / self.height as f32;
                let vx = SnowRenderer::perlin3(nx, ny, t);
                let vy = SnowRenderer::perlin3(nx + 37.0, ny + 11.0, t + 5.3);
                let idx = self.index(x, y);
                self.data[idx] = [vx, vy];
            }
        }
        self.add_cursor_impulse(cursor_pos, cursor_delta, viewport);
    }

    fn add_cursor_impulse(&mut self, cursor_pos: [f32; 2], cursor_delta: [f32; 2], viewport: [f32; 2]) {
        let radius = (self.width.min(self.height) as f32 * 0.2).max(3.0);
        let falloff = 1.0 / (radius * radius);
        let cx = (cursor_pos[0] / viewport[0].max(1.0)) * self.width as f32;
        let cy = (cursor_pos[1] / viewport[1].max(1.0)) * self.height as f32;
        for y in 0..self.height {
            for x in 0..self.width {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let dist2 = dx * dx + dy * dy;
                if dist2 > radius * radius {
                    continue;
                }
                let w = (1.0 - dist2 * falloff).clamp(0.0, 1.0);
                let idx = self.index(x, y);
                self.data[idx][0] += cursor_delta[0] * w * 0.2;
                self.data[idx][1] += cursor_delta[1] * w * 0.2;
            }
        }
    }

    fn sample(&self, x: f32, y: f32, viewport: [f32; 2]) -> [f32; 2] {
        if self.width == 0 || self.height == 0 {
            return [0.0, 0.0];
        }
        let clamped_x = x.clamp(0.0, viewport[0].max(1.0));
        let clamped_y = y.clamp(0.0, viewport[1].max(1.0));
        let fx = (clamped_x / viewport[0].max(1.0)) * (self.width as f32 - 1.001);
        let fy = (clamped_y / viewport[1].max(1.0)) * (self.height as f32 - 1.001);
        let x0 = fx.floor() as usize;
        let y0 = fy.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;

        let v00 = self.data[self.index(x0, y0)];
        let v10 = self.data[self.index(x1, y0)];
        let v01 = self.data[self.index(x0, y1)];
        let v11 = self.data[self.index(x1, y1)];

        let vx0 = v00[0] * (1.0 - tx) + v10[0] * tx;
        let vx1 = v01[0] * (1.0 - tx) + v11[0] * tx;
        let vy0 = v00[1] * (1.0 - tx) + v10[1] * tx;
        let vy1 = v01[1] * (1.0 - tx) + v11[1] * tx;

        [
            vx0 * (1.0 - ty) + vx1 * ty,
            vy0 * (1.0 - ty) + vy1 * ty,
        ]
    }

    #[inline]
    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }
}

impl SnowRenderer {
    fn new(width: f32, height: f32) -> Result<Self, JsValue> {
        if MAX_PARTICLES > PARTICLE_CAPACITY {
            return Err(js_err("particle capacity must be >= MAX_PARTICLES"));
        }

        let window = web_sys::window().ok_or_else(|| js_err("missing window"))?;
        let document = window
            .document()
            .ok_or_else(|| js_err("missing document"))?;

        let canvas = document
            .get_element_by_id("snow-canvas")
            .ok_or_else(|| js_err("missing #snow-canvas element"))?
            .dyn_into::<HtmlCanvasElement>()?;

        canvas.set_width(width as u32);
        canvas.set_height(height as u32);

        let gl: WebGl2RenderingContext = canvas
            .get_context("webgl2")?
            .ok_or_else(|| js_err("WebGL2 not supported"))?
            .dyn_into()?;

        gl.get_extension("EXT_color_buffer_float")
            .map_err(|_| js_err("unable to enable EXT_color_buffer_float"))?;

        gl.enable(WebGl2RenderingContext::BLEND);
        gl.blend_func(
            WebGl2RenderingContext::SRC_ALPHA,
            WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA,
        );
        gl.disable(WebGl2RenderingContext::DEPTH_TEST);
        gl.disable(WebGl2RenderingContext::CULL_FACE);

        let render_program =
            Self::link_program(&gl, shaders::RENDER_VERTEX, shaders::RENDER_FRAGMENT)?;

        let render_vao = gl
            .create_vertex_array()
            .ok_or_else(|| js_err("failed to create render VAO"))?;
        gl.bind_vertex_array(Some(&render_vao));

        let particle_buffer = gl
            .create_buffer()
            .ok_or_else(|| js_err("failed to create particle buffer"))?;
        gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&particle_buffer));

        let uv_data = Self::generate_particle_uvs();
        unsafe {
            let view = Float32Array::view(&uv_data);
            gl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &view,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }
        gl.vertex_attrib_pointer_with_i32(0, 2, WebGl2RenderingContext::FLOAT, false, 0, 0);
        gl.enable_vertex_attrib_array(0);
        gl.bind_vertex_array(None);

        let mut rng_state = 0x5EED_A55E;
        let initial_data = Self::initial_texture_data(width, height, &mut rng_state);
        let texture = Self::upload_texture(&gl, &initial_data)?;
        let particle_data = initial_data.clone();
        let particle_velocity = vec![[0.0f32; 2]; MAX_PARTICLES];
        let pile_bins = vec![0.0; PILE_BINS];
        let pile_smoothed = vec![0.0; PILE_BINS];
        let flow_field = FlowField::new(FLOW_WIDTH, FLOW_HEIGHT);
        let spawn_accum_ms = 0.0;
        let next_spawn_ms = 8.0;
        let spawn_cursor = PILE_BINS;

        let mut renderer = SnowRenderer {
            canvas,
            gl,
            render_program,
            render_vao,
            _particle_buffer: particle_buffer,
            texture,
            width,
            height,
            pointer_target: 0.0,
            pointer_current: 0.0,
            pointer_pos: [width * 0.5, height * 0.5],
            pointer_delta: [0.0, 0.0],
            time_ms: 0.0,
            rng_state,
            tint: [1.0, 1.0, 1.0],
            texture_size: [TEXTURE_WIDTH as f32, TEXTURE_HEIGHT as f32],
            particle_velocity,
            particle_data,
            pile_bins,
            pile_smoothed,
            flow_field,
            spawn_accum_ms,
            next_spawn_ms,
            spawn_cursor,
            render_uniforms: RenderUniforms::default(),
        };

        renderer.render_uniforms = renderer.cache_render_uniforms()?;
        renderer.apply_static_uniforms();

        Ok(renderer)
    }

    fn cache_render_uniforms(&self) -> Result<RenderUniforms, JsValue> {
        Ok(RenderUniforms {
            particles: self
                .gl
                .get_uniform_location(&self.render_program, "u_particles"),
            viewport: self
                .gl
                .get_uniform_location(&self.render_program, "u_viewport"),
            texture_size: self
                .gl
                .get_uniform_location(&self.render_program, "u_textureSize"),
            point_scale: self
                .gl
                .get_uniform_location(&self.render_program, "u_pointScale"),
            tint: self.gl.get_uniform_location(&self.render_program, "u_tint"),
        })
    }

    fn apply_static_uniforms(&self) {
        self.gl.use_program(Some(&self.render_program));
        if let Some(particles) = &self.render_uniforms.particles {
            self.gl.uniform1i(Some(particles), 0);
        }
        if let Some(texture_size) = &self.render_uniforms.texture_size {
            self.gl.uniform2f(
                Some(texture_size),
                self.texture_size[0],
                self.texture_size[1],
            );
        }
        if let Some(tint) = &self.render_uniforms.tint {
            self.gl
                .uniform3f(Some(tint), self.tint[0], self.tint[1], self.tint[2]);
        }
    }

    fn resize(&mut self, width: f32, height: f32) -> Result<(), JsValue> {
        self.width = width.max(1.0);
        self.height = height.max(1.0);
        self.canvas.set_width(self.width as u32);
        self.canvas.set_height(self.height as u32);
        Ok(())
    }

    fn reset(&mut self) -> Result<(), JsValue> {
        self.pointer_target = 0.0;
        self.pointer_current = 0.0;
        self.pointer_pos = [self.width * 0.5, self.height * 0.5];
        self.pointer_delta = [0.0, 0.0];
        self.time_ms = 0.0;
        self.rng_state = 0x5EED_A55E;
        self.flow_field.clear();
        self.pile_bins.fill(0.0);
        self.pile_smoothed.fill(0.0);
        self.particle_velocity.fill([0.0, 0.0]);
        self.particle_data =
            Self::initial_texture_data(self.width, self.height, &mut self.rng_state);
        let data = self.particle_data.clone();
        self.upload_texture_data(&self.texture, &data)?;
        self.spawn_accum_ms = 0.0;
        self.next_spawn_ms = 8.0;
        self.spawn_cursor = PILE_BINS;
        Ok(())
    }

    fn step(&mut self, delta_ms: f32) -> Result<(), JsValue> {
        if delta_ms <= 0.0 {
            return Ok(());
        }
        let clamped = delta_ms.min(1000.0);
        self.time_ms += clamped;
        let smoothing = (clamped / FIXED_TIMESTEP).clamp(0.05, 1.0) * 0.15;
        self.pointer_current += (self.pointer_target - self.pointer_current) * smoothing;
        self.cpu_particle_step(clamped)?;
        self.upload_texture_data(&self.texture, &self.particle_data)?;
        self.render_pass()?;
        Ok(())
    }

    fn cpu_particle_step(&mut self, delta_ms: f32) -> Result<(), JsValue> {
        let dt = (delta_ms * 0.001).min(0.1);
        let width = self.width.max(1.0);
        let height = self.height.max(1.0);
        let bin_scale = (PILE_BINS - 1) as f32 / width;
        let cursor_bias = ((self.pointer_pos[0] / width).clamp(0.0, 1.0) - 0.5) * 40.0;
        let wind = self.pointer_current * 60.0 + self.pointer_delta[0] * 0.25 + cursor_bias;
        let gravity = 120.0;
        let drag = 0.8;
        let flow_scale = 95.0;
        self.flow_field
            .update(self.time_ms * 0.001, [width, height], self.pointer_pos, self.pointer_delta);

        self.spawn_accum_ms += delta_ms;
        while self.spawn_accum_ms >= self.next_spawn_ms {
            self.spawn_accum_ms -= self.next_spawn_ms;
            self.spawn_one(width, height);
            self.next_spawn_ms = Self::rand_range(&mut self.rng_state, 4.0, 16.0);
        }

        // Falling particles start after the reserved pile marker slots.
        for slot in PILE_BINS..MAX_PARTICLES {
            let base = slot * TEXEL_STRIDE;
            if self.particle_data[base + 2] <= 0.01 {
                continue; // inactive, awaiting spawn
            }
            let radius = self.particle_data[base + 2].max(0.5);
            let flow = self
                .flow_field
                .sample(self.particle_data[base], self.particle_data[base + 1], [width, height]);
            let accel_x = flow[0] * flow_scale + wind;
            let accel_y = flow[1] * flow_scale + gravity;
            let vx = self.particle_velocity[slot][0] + (accel_x - self.particle_velocity[slot][0] * drag) * dt;
            let vy = self.particle_velocity[slot][1] + (accel_y - self.particle_velocity[slot][1] * drag) * dt;
            self.particle_velocity[slot] = [vx, vy];

            let mut x = self.particle_data[base] + vx * dt;
            let mut y = self.particle_data[base + 1] + vy * dt;

            // Wrap horizontally.
            if x < -20.0 {
                x += width + 40.0;
            } else if x > width + 20.0 {
                x -= width + 40.0;
            }

            let bin = ((x.clamp(0.0, width - 1.0)) * bin_scale).round() as usize;
            let pile_height = self.pile_smoothed[bin];
            let ground_y = height - pile_height;

            if y + radius >= ground_y {
                self.pile_bins[bin] = (self.pile_bins[bin] + radius * 1.1).min(height * 0.8);
                // Mark inactive; respawn later via spawn loop.
                self.particle_data[base + 2] = 0.0;
                self.particle_data[base + 3] = 0.0;
                self.particle_velocity[slot] = [0.0, 0.0];
            }

            self.particle_data[base] = x;
            self.particle_data[base + 1] = y;
        }

        self.smooth_pile();
        self.update_pile_particles(height, width);
        self.pointer_delta = [0.0, 0.0];
        Ok(())
    }

    fn smooth_pile(&mut self) {
        let len = self.pile_bins.len();
        if len == 0 {
            return;
        }

        // Stronger Gaussian smoothing for a soft snow mound.
        let radius = 8;
        let sigma = 3.5;
        let mut weights = Vec::with_capacity(radius * 2 + 1);
        let mut w_sum = 0.0;
        for i in -(radius as isize)..=(radius as isize) {
            let w = (-0.5 * (i as f32 / sigma).powi(2)).exp();
            weights.push(w);
            w_sum += w;
        }

        let mut next = vec![0.0f32; len];
        for i in 0..len {
            let mut acc = 0.0;
            for (offset, w) in (-(radius as isize)..=(radius as isize)).zip(weights.iter()) {
                let idx = (i as isize + offset).clamp(0, len as isize - 1) as usize;
                acc += self.pile_bins[idx] * *w;
            }
            next[i] = acc / w_sum;
        }

        // Gentle settling to keep the pile from growing without bound.
        for v in &mut next {
            *v *= 0.999;
        }
        for v in &mut self.pile_bins {
            *v *= 0.9995;
        }
        self.pile_smoothed.copy_from_slice(&next);
    }

    fn update_pile_particles(&mut self, height: f32, width: f32) {
        for bin in 0..PILE_BINS {
            let base = bin * TEXEL_STRIDE;
            let pile_h = self.pile_smoothed[bin].min(height);
            let x = (bin as f32 + 0.5) / PILE_BINS as f32 * width;
            let y = height - pile_h;
            self.particle_data[base] = x;
            self.particle_data[base + 1] = y;
            self.particle_data[base + 2] = 0.0; // hide pile sprites; pile is drawn in overlay
            self.particle_data[base + 3] = 0.0;
        }
    }

    fn spawn_one(&mut self, width: f32, _height: f32) {
        let slot = self.spawn_cursor;
        let base = slot * TEXEL_STRIDE;
        let radius = Self::rand_range(&mut self.rng_state, 0.7, 3.6);
        let base_rand = Self::rand(&mut self.rng_state);
        let spread = Self::rand(&mut self.rng_state);
        let x = base_rand * (width + 40.0) - 20.0;
        let y = -radius - spread * 80.0;
        self.particle_data[base] = x;
        self.particle_data[base + 1] = y;
        self.particle_data[base + 2] = radius;
        self.particle_data[base + 3] = Self::rand(&mut self.rng_state);
        self.particle_velocity[slot] = [0.0, 0.0];
        self.spawn_cursor += 1;
        if self.spawn_cursor >= MAX_PARTICLES {
            self.spawn_cursor = PILE_BINS;
        }
    }

    fn render_pass(&self) -> Result<(), JsValue> {
        self.gl.bind_vertex_array(Some(&self.render_vao));
        self.gl.use_program(Some(&self.render_program));
        self.gl
            .bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
        self.gl.viewport(
            0,
            0,
            self.canvas.width() as i32,
            self.canvas.height() as i32,
        );
        self.gl.clear_color(0.0, 0.0, 0.0, 0.0);
        self.gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
        self.gl.active_texture(WebGl2RenderingContext::TEXTURE0);
        self.gl.bind_texture(
            WebGl2RenderingContext::TEXTURE_2D,
            Some(&self.texture),
        );

        if let Some(viewport) = &self.render_uniforms.viewport {
            self.gl.uniform2f(Some(viewport), self.width, self.height);
        }
        if let Some(point_scale) = &self.render_uniforms.point_scale {
            self.gl.uniform1f(Some(point_scale), self.point_scale());
        }
        if let Some(tint) = &self.render_uniforms.tint {
            self.gl
                .uniform3f(Some(tint), self.tint[0], self.tint[1], self.tint[2]);
        }

        self.gl
            .draw_arrays(WebGl2RenderingContext::POINTS, 0, MAX_PARTICLES as i32);

        Ok(())
    }

    fn point_scale(&self) -> f32 {
        let canvas_height = self.canvas.height() as f32;
        if self.height <= 0.0 {
            return 2.0;
        }
        (canvas_height / self.height).max(1.0) * 2.0
    }

    fn set_tint(&mut self, tint: [f32; 3]) {
        self.tint = tint;
        if let Some(location) = &self.render_uniforms.tint {
            self.gl.use_program(Some(&self.render_program));
            self.gl
                .uniform3f(Some(location), self.tint[0], self.tint[1], self.tint[2]);
        }
    }

    fn upload_texture_data(&self, texture: &WebGlTexture, data: &[f32]) -> Result<(), JsValue> {
        self.gl
            .bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(texture));
        unsafe {
            let view = Float32Array::view(data);
            self.gl
                .tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                    WebGl2RenderingContext::TEXTURE_2D,
                    0,
                    0,
                    0,
                    TEXTURE_WIDTH as i32,
                    TEXTURE_HEIGHT as i32,
                    WebGl2RenderingContext::RGBA,
                    WebGl2RenderingContext::FLOAT,
                    Some(&view),
                )?;
        }
        Ok(())
    }

    fn initial_texture_data(width: f32, height: f32, _rng_state: &mut u32) -> Vec<f32> {
        let mut data = vec![0.0; TEXTURE_FLOATS];
        for slot in 0..PARTICLE_CAPACITY {
            let base = slot * TEXEL_STRIDE;
            if slot < PILE_BINS {
                data[base] = (slot as f32 + 0.5) / PILE_BINS as f32 * width.max(1.0);
                data[base + 1] = height.max(1.0);
                data[base + 2] = 0.0;
                data[base + 3] = 0.0;
            } else if slot < MAX_PARTICLES {
                data[base] = 0.0;
                data[base + 1] = 0.0;
                data[base + 2] = 0.0;
                data[base + 3] = 0.0;
            }
        }
        data
    }

    fn generate_particle_uvs() -> Vec<f32> {
        let mut data = vec![0.0f32; MAX_PARTICLES * 2];
        for index in 0..MAX_PARTICLES {
            let u = (index % TEXTURE_WIDTH) as f32 + 0.5;
            let v = (index / TEXTURE_WIDTH) as f32 + 0.5;
            data[index * 2] = u / TEXTURE_WIDTH as f32;
            data[index * 2 + 1] = v / TEXTURE_HEIGHT as f32;
        }
        data
    }

    fn upload_texture(gl: &WebGl2RenderingContext, data: &[f32]) -> Result<WebGlTexture, JsValue> {
        let texture = gl
            .create_texture()
            .ok_or_else(|| js_err("failed to create texture"))?;
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&texture));
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MIN_FILTER,
            WebGl2RenderingContext::NEAREST as i32,
        );
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MAG_FILTER,
            WebGl2RenderingContext::NEAREST as i32,
        );
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_WRAP_S,
            WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_WRAP_T,
            WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
        );
        unsafe {
            let view = Float32Array::view(data);
            gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
                WebGl2RenderingContext::TEXTURE_2D,
                0,
                WebGl2RenderingContext::RGBA32F as i32,
                TEXTURE_WIDTH as i32,
                TEXTURE_HEIGHT as i32,
                0,
                WebGl2RenderingContext::RGBA,
                WebGl2RenderingContext::FLOAT,
                Some(&view),
            )?;
        }
        Ok(texture)
    }

    fn rand(state: &mut u32) -> f32 {
        *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        let value = (*state >> 8) as f32;
        value / ((u32::MAX >> 8) as f32)
    }

    fn rand_range(state: &mut u32, min: f32, max: f32) -> f32 {
        min + (max - min) * Self::rand(state)
    }

    fn hash3(mut x: i32, mut y: i32, mut z: i32) -> u32 {
        x = x.wrapping_mul(374761393);
        y = y.wrapping_mul(668265263);
        z = z.wrapping_mul(982451653);
        let mut h = x ^ y ^ z;
        h ^= h >> 13;
        h = h.wrapping_mul(1274126177);
        (h ^ (h >> 16)) as u32
    }

    fn fade(t: f32) -> f32 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    fn grad(hash: u32, x: f32, y: f32, z: f32) -> f32 {
        let h = hash & 15;
        let u = if h < 8 { x } else { y };
        let v = if h < 4 {
            y
        } else if h == 12 || h == 14 {
            x
        } else {
            z
        };
        let sign_u = if (h & 1) == 0 { u } else { -u };
        let sign_v = if (h & 2) == 0 { v } else { -v };
        sign_u + sign_v
    }

    fn perlin3(x: f32, y: f32, z: f32) -> f32 {
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let z0 = z.floor() as i32;
        let x_rel = x - x0 as f32;
        let y_rel = y - y0 as f32;
        let z_rel = z - z0 as f32;

        let u = Self::fade(x_rel);
        let v = Self::fade(y_rel);
        let w = Self::fade(z_rel);

        let aaa = Self::hash3(x0, y0, z0);
        let aba = Self::hash3(x0, y0 + 1, z0);
        let aab = Self::hash3(x0, y0, z0 + 1);
        let abb = Self::hash3(x0, y0 + 1, z0 + 1);
        let baa = Self::hash3(x0 + 1, y0, z0);
        let bba = Self::hash3(x0 + 1, y0 + 1, z0);
        let bab = Self::hash3(x0 + 1, y0, z0 + 1);
        let bbb = Self::hash3(x0 + 1, y0 + 1, z0 + 1);

        let x1 = Self::grad(aaa, x_rel, y_rel, z_rel);
        let x2 = Self::grad(baa, x_rel - 1.0, y_rel, z_rel);
        let y1 = x1 + u * (x2 - x1);

        let x3 = Self::grad(aba, x_rel, y_rel - 1.0, z_rel);
        let x4 = Self::grad(bba, x_rel - 1.0, y_rel - 1.0, z_rel);
        let y2 = x3 + u * (x4 - x3);

        let z1 = y1 + v * (y2 - y1);

        let x5 = Self::grad(aab, x_rel, y_rel, z_rel - 1.0);
        let x6 = Self::grad(bab, x_rel - 1.0, y_rel, z_rel - 1.0);
        let y3 = x5 + u * (x6 - x5);

        let x7 = Self::grad(abb, x_rel, y_rel - 1.0, z_rel - 1.0);
        let x8 = Self::grad(bbb, x_rel - 1.0, y_rel - 1.0, z_rel - 1.0);
        let y4 = x7 + u * (x8 - x7);

        let z2 = y3 + v * (y4 - y3);

        (z1 + w * (z2 - z1)).clamp(-1.0, 1.0)
    }

    fn link_program(
        gl: &WebGl2RenderingContext,
        vertex_src: &str,
        fragment_src: &str,
    ) -> Result<WebGlProgram, JsValue> {
        let vertex_shader =
            Self::compile_shader(gl, WebGl2RenderingContext::VERTEX_SHADER, vertex_src)?;
        let fragment_shader =
            Self::compile_shader(gl, WebGl2RenderingContext::FRAGMENT_SHADER, fragment_src)?;
        let program = gl
            .create_program()
            .ok_or_else(|| js_err("failed to create program"))?;
        gl.attach_shader(&program, &vertex_shader);
        gl.attach_shader(&program, &fragment_shader);
        gl.link_program(&program);
        if gl
            .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            gl.detach_shader(&program, &vertex_shader);
            gl.detach_shader(&program, &fragment_shader);
            gl.delete_shader(Some(&vertex_shader));
            gl.delete_shader(Some(&fragment_shader));
            Ok(program)
        } else {
            let info = gl
                .get_program_info_log(&program)
                .unwrap_or_else(|| "unknown program error".to_string());
            Err(js_err(&format!("failed to link program: {}", info)))
        }
    }

    fn compile_shader(
        gl: &WebGl2RenderingContext,
        shader_type: u32,
        source: &str,
    ) -> Result<WebGlShader, JsValue> {
        let shader = gl
            .create_shader(shader_type)
            .ok_or_else(|| js_err("failed to create shader"))?;
        gl.shader_source(&shader, source);
        gl.compile_shader(&shader);
        if gl
            .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            Ok(shader)
        } else {
            let info = gl
                .get_shader_info_log(&shader)
                .unwrap_or_else(|| "unknown shader error".to_string());
            Err(js_err(&format!("failed to compile shader: {}", info)))
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn particles_move_with_wind() {
        let mut renderer = SnowRenderer::new(100.0, 100.0).unwrap();
        let before = renderer.particle_data[PILE_BINS * TEXEL_STRIDE];
        renderer.pointer_target = 0.4;
        renderer.step(16.0).unwrap();
        let after = renderer.particle_data[PILE_BINS * TEXEL_STRIDE];
        assert_ne!(before, after);
    }

    #[wasm_bindgen_test]
    fn pile_grows_on_landings() {
        let mut renderer = SnowRenderer::new(50.0, 50.0).unwrap();
        let idx = (PILE_BINS + 1) * TEXEL_STRIDE;
        renderer.particle_data[idx + 1] = 60.0; // force landing next step
        renderer.step(16.0).unwrap();
        let total: f32 = renderer.pile_bins.iter().sum();
        assert!(total > 0.0);
    }
}
