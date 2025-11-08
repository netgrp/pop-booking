mod shaders;

use js_sys::Float32Array;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext, WebGlBuffer, WebGlFramebuffer, WebGlProgram,
    WebGlShader, WebGlTexture, WebGlUniformLocation, WebGlVertexArrayObject,
};

const TARGET_FPS: f32 = 60.0;
const FIXED_TIMESTEP: f32 = 1000.0 / TARGET_FPS;
const MAX_PARTICLES: usize = 10_000;
const TEXTURE_WIDTH: usize = 128;
const TEXTURE_HEIGHT: usize = 128;
const PARTICLE_CAPACITY: usize = TEXTURE_WIDTH * TEXTURE_HEIGHT;
const TEXEL_STRIDE: usize = 4;
const TEXTURE_FLOATS: usize = PARTICLE_CAPACITY * TEXEL_STRIDE;

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
    compute_program: WebGlProgram,
    render_program: WebGlProgram,
    quad_vao: WebGlVertexArrayObject,
    render_vao: WebGlVertexArrayObject,
    _particle_buffer: WebGlBuffer,
    framebuffer: WebGlFramebuffer,
    textures: [WebGlTexture; 2],
    current_src: usize,
    width: f32,
    height: f32,
    pointer_target: f32,
    pointer_current: f32,
    time_ms: f32,
    rng_state: u32,
    tint: [f32; 3],
    texture_size: [f32; 2],
    compute_uniforms: ComputeUniforms,
    render_uniforms: RenderUniforms,
}

#[derive(Default)]
struct ComputeUniforms {
    source: Option<WebGlUniformLocation>,
    viewport: Option<WebGlUniformLocation>,
    texture_size: Option<WebGlUniformLocation>,
    delta_ms: Option<WebGlUniformLocation>,
    time: Option<WebGlUniformLocation>,
    wind: Option<WebGlUniformLocation>,
}

#[derive(Default)]
struct RenderUniforms {
    particles: Option<WebGlUniformLocation>,
    viewport: Option<WebGlUniformLocation>,
    texture_size: Option<WebGlUniformLocation>,
    point_scale: Option<WebGlUniformLocation>,
    tint: Option<WebGlUniformLocation>,
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

        let compute_program =
            Self::link_program(&gl, shaders::COMPUTE_VERTEX, shaders::COMPUTE_FRAGMENT)?;
        let render_program =
            Self::link_program(&gl, shaders::RENDER_VERTEX, shaders::RENDER_FRAGMENT)?;

        let quad_vao = gl
            .create_vertex_array()
            .ok_or_else(|| js_err("failed to create compute VAO"))?;
        gl.bind_vertex_array(Some(&quad_vao));
        gl.bind_vertex_array(None);

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

        let framebuffer = gl
            .create_framebuffer()
            .ok_or_else(|| js_err("failed to create framebuffer"))?;

        let mut rng_state = 0x5EED_A55E;
        let initial_data = Self::initial_texture_data(width, height, &mut rng_state);
        let textures = [
            Self::upload_texture(&gl, &initial_data)?,
            Self::upload_texture(&gl, &initial_data)?,
        ];

        let mut renderer = SnowRenderer {
            canvas,
            gl,
            compute_program,
            render_program,
            quad_vao,
            render_vao,
            _particle_buffer: particle_buffer,
            framebuffer,
            textures,
            current_src: 0,
            width,
            height,
            pointer_target: 0.0,
            pointer_current: 0.0,
            time_ms: 0.0,
            rng_state,
            tint: [1.0, 1.0, 1.0],
            texture_size: [TEXTURE_WIDTH as f32, TEXTURE_HEIGHT as f32],
            compute_uniforms: ComputeUniforms::default(),
            render_uniforms: RenderUniforms::default(),
        };

        renderer.compute_uniforms = renderer.cache_compute_uniforms()?;
        renderer.render_uniforms = renderer.cache_render_uniforms()?;
        renderer.apply_static_uniforms();

        Ok(renderer)
    }

    fn cache_compute_uniforms(&self) -> Result<ComputeUniforms, JsValue> {
        Ok(ComputeUniforms {
            source: self
                .gl
                .get_uniform_location(&self.compute_program, "u_source"),
            viewport: self
                .gl
                .get_uniform_location(&self.compute_program, "u_viewport"),
            texture_size: self
                .gl
                .get_uniform_location(&self.compute_program, "u_textureSize"),
            delta_ms: self
                .gl
                .get_uniform_location(&self.compute_program, "u_deltaMs"),
            time: self
                .gl
                .get_uniform_location(&self.compute_program, "u_time"),
            wind: self
                .gl
                .get_uniform_location(&self.compute_program, "u_wind"),
        })
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
        self.gl.use_program(Some(&self.compute_program));
        if let Some(source) = &self.compute_uniforms.source {
            self.gl.uniform1i(Some(source), 0);
        }
        if let Some(texture_size) = &self.compute_uniforms.texture_size {
            self.gl.uniform2f(
                Some(texture_size),
                self.texture_size[0],
                self.texture_size[1],
            );
        }

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
        self.time_ms = 0.0;
        self.rng_state = 0x5EED_A55E;
        let data = Self::initial_texture_data(self.width, self.height, &mut self.rng_state);
        for texture in &self.textures {
            self.upload_texture_data(texture, &data)?;
        }
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
        self.run_compute_pass(clamped)?;
        self.render_pass()?;
        Ok(())
    }

    fn run_compute_pass(&mut self, delta_ms: f32) -> Result<(), JsValue> {
        let target = 1 - self.current_src;
        self.gl.bind_vertex_array(Some(&self.quad_vao));
        self.gl.use_program(Some(&self.compute_program));
        self.gl
            .bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, Some(&self.framebuffer));
        self.gl.framebuffer_texture_2d(
            WebGl2RenderingContext::FRAMEBUFFER,
            WebGl2RenderingContext::COLOR_ATTACHMENT0,
            WebGl2RenderingContext::TEXTURE_2D,
            Some(&self.textures[target]),
            0,
        );
        self.gl
            .viewport(0, 0, TEXTURE_WIDTH as i32, TEXTURE_HEIGHT as i32);

        self.gl.active_texture(WebGl2RenderingContext::TEXTURE0);
        self.gl.bind_texture(
            WebGl2RenderingContext::TEXTURE_2D,
            Some(&self.textures[self.current_src]),
        );

        if let Some(viewport) = &self.compute_uniforms.viewport {
            self.gl.uniform2f(Some(viewport), self.width, self.height);
        }
        if let Some(delta) = &self.compute_uniforms.delta_ms {
            self.gl.uniform1f(Some(delta), delta_ms);
        }
        if let Some(time) = &self.compute_uniforms.time {
            self.gl.uniform1f(Some(time), self.time_ms);
        }
        if let Some(wind) = &self.compute_uniforms.wind {
            self.gl.uniform1f(Some(wind), self.pointer_current);
        }

        self.gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 3);
        self.gl
            .bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
        self.current_src = target;
        Ok(())
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
            Some(&self.textures[self.current_src]),
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

    fn initial_texture_data(width: f32, height: f32, rng_state: &mut u32) -> Vec<f32> {
        let mut data = vec![0.0; TEXTURE_FLOATS];
        for slot in 0..PARTICLE_CAPACITY {
            let base = slot * TEXEL_STRIDE;
            if slot < MAX_PARTICLES {
                let radius = Self::rand_range(rng_state, 0.8, 2.8);
                data[base] = Self::rand(rng_state) * width.max(1.0);
                data[base + 1] = -radius - Self::rand(rng_state) * height.max(1.0);
                data[base + 2] = radius;
                data[base + 3] = Self::rand(rng_state);
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
