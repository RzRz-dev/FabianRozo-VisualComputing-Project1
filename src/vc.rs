// vc.rs -- thin layer for the Rust host, mirroring twgl's surface:
// create_program_info / create_buffer_info / set_uniform /
// draw_buffer_info, plus the column-major mat4 helpers twgl's m4
// module supplies. glow provides the GL bindings; every call sits in
// an unsafe block because glow forwards the raw API, unchecked. (The
// math half is hand-rolled to keep the crate lean; glam is the
// ecosystem's canonical reach when a program outgrows these four
// functions.)

use glow::HasContext;
use std::collections::HashMap;

pub type Mat4 = [f32; 16]; // column-major
pub type Mat3 = [f32; 9]; // column-major

pub struct ProgramInfo {
    pub program: glow::Program,
    pub uniforms: HashMap<String, glow::UniformLocation>,
}

pub struct BufferInfo {
    pub vao: glow::VertexArray,
    pub num_elements: i32,
    pub indexed: bool,
}

pub struct Attrib<'a> {
    pub name: &'a str, // as the shader declares it
    pub num_components: i32,
    pub data: &'a [f32],
}

pub fn create_program_info(
    gl: &glow::Context,
    vert_src: &str,
    frag_src: &str,
) -> ProgramInfo {
    unsafe {
        let compile = |kind, src: &str| {
            let sh = gl.create_shader(kind).unwrap();
            gl.shader_source(sh, src);
            gl.compile_shader(sh);
            if !gl.get_shader_compile_status(sh) {
                panic!("{}", gl.get_shader_info_log(sh));
            }
            sh
        };
        let program = gl.create_program().unwrap();
        gl.attach_shader(program, compile(glow::VERTEX_SHADER, vert_src));
        gl.attach_shader(program, compile(glow::FRAGMENT_SHADER, frag_src));
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            panic!("{}", gl.get_program_info_log(program));
        }
        let mut uniforms = HashMap::new();
        for i in 0..gl.get_active_uniforms(program) {
            let u = gl.get_active_uniform(program, i).unwrap();
            if let Some(loc) = gl.get_uniform_location(program, &u.name) {
                uniforms.insert(u.name, loc);
            }
        }
        ProgramInfo { program, uniforms }
    }
}

pub fn create_buffer_info(
    gl: &glow::Context,
    info: &ProgramInfo,
    attribs: &[Attrib],
    indices: Option<&[u16]>,
) -> BufferInfo {
    unsafe {
        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));
        for a in attribs {
            let vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let bytes = core::slice::from_raw_parts(
                a.data.as_ptr() as *const u8,
                a.data.len() * 4,
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes,
                                    glow::STATIC_DRAW);
            let loc = gl.get_attrib_location(info.program, a.name).unwrap();
            gl.enable_vertex_attrib_array(loc);
            gl.vertex_attrib_pointer_f32(loc, a.num_components, glow::FLOAT,
                                         false, 0, 0);
        }
        match indices {
            Some(idx) => {
                let ibo = gl.create_buffer().unwrap();
                gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ibo));
                let bytes = core::slice::from_raw_parts(
                    idx.as_ptr() as *const u8,
                    idx.len() * 2,
                );
                gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, bytes,
                                        glow::STATIC_DRAW);
                BufferInfo { vao, num_elements: idx.len() as i32,
                             indexed: true }
            }
            None => BufferInfo {
                vao,
                num_elements: (attribs[0].data.len() as i32)
                    / attribs[0].num_components,
                indexed: false,
            },
        }
    }
}

pub fn set_uniform(
    gl: &glow::Context,
    info: &ProgramInfo,
    name: &str,
    m: &Mat4,
) {
    unsafe {
        gl.uniform_matrix_4_f32_slice(Some(&info.uniforms[name]), false, m);
    }
}

pub fn draw_buffer_info(gl: &glow::Context, buf: &BufferInfo) {
    unsafe {
        gl.bind_vertex_array(Some(buf.vao));
        if buf.indexed {
            gl.draw_elements(glow::TRIANGLES, buf.num_elements,
                             glow::UNSIGNED_SHORT, 0);
        } else {
            gl.draw_arrays(glow::TRIANGLES, 0, buf.num_elements);
        }
    }
}

pub fn set_uniform_mat3(
    gl: &glow::Context,
    info: &ProgramInfo,
    name: &str,
    m: &Mat3,
) {
    unsafe {
        gl.uniform_matrix_3_f32_slice(Some(&info.uniforms[name]), false, m);
    }
}

pub fn set_uniform_f32(
    gl: &glow::Context,
    info: &ProgramInfo,
    name: &str,
    v: f32,
) {
    unsafe {
        gl.uniform_1_f32(Some(&info.uniforms[name]), v);
    }
}

pub fn set_uniform_i32(
    gl: &glow::Context,
    info: &ProgramInfo,
    name: &str,
    v: i32,
) {
    unsafe {
        gl.uniform_1_i32(Some(&info.uniforms[name]), v);
    }
}

pub fn set_uniform_vec3(
    gl: &glow::Context,
    info: &ProgramInfo,
    name: &str,
    v: &[f32; 3],
) {
    unsafe {
        gl.uniform_3_f32_slice(Some(&info.uniforms[name]), v);
    }
}

pub fn set_uniform_vec2(
    gl: &glow::Context,
    info: &ProgramInfo,
    name: &str,
    v: &[f32; 2],
) {
    unsafe {
        gl.uniform_2_f32_slice(Some(&info.uniforms[name]), v);
    }
}

// ---- textures -----------------------------------------------------------

pub struct Texture {
    pub id: glow::Texture,
    pub width: i32,
    pub height: i32,
}

// RGBA8 texture from raw pixels (row 0 = bottom, GL's layout), linear
// filtering, clamped edges. Pass None to allocate uninitialized
// storage (render-target use).
pub fn create_texture(
    gl: &glow::Context,
    width: i32,
    height: i32,
    pixels: Option<&[u8]>,
) -> Texture {
    unsafe {
        let id = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D, Some(id));
        gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, width,
                        height, 0, glow::RGBA, glow::UNSIGNED_BYTE, pixels);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER,
                             glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER,
                             glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S,
                             glow::CLAMP_TO_EDGE as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T,
                             glow::CLAMP_TO_EDGE as i32);
        Texture { id, width, height }
    }
}

// Bind the texture to a unit and point the sampler uniform at it.
pub fn set_texture(
    gl: &glow::Context,
    info: &ProgramInfo,
    name: &str,
    tex: &Texture,
    unit: u32,
) {
    unsafe {
        gl.active_texture(glow::TEXTURE0 + unit);
        gl.bind_texture(glow::TEXTURE_2D, Some(tex.id));
        gl.uniform_1_i32(Some(&info.uniforms[name]), unit as i32);
    }
}

// ---- framebuffers -------------------------------------------------------

pub struct Framebuffer {
    pub fb: glow::Framebuffer,
    pub color: Texture,          // sampleable color attachment
    pub depth: Option<Texture>,  // sampleable depth attachment, on request
}

// Offscreen target: a color texture attachment, plus either a
// sampleable depth *texture* (with_depth_texture) or a plain depth
// renderbuffer.
pub fn create_framebuffer(
    gl: &glow::Context,
    width: i32,
    height: i32,
    with_depth_texture: bool,
) -> Framebuffer {
    unsafe {
        let color = create_texture(gl, width, height, None);
        let fb = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fb));
        gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0,
                                  glow::TEXTURE_2D, Some(color.id), 0);
        let depth = if with_depth_texture {
            let id = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(id));
            gl.tex_image_2d(glow::TEXTURE_2D, 0,
                            glow::DEPTH_COMPONENT24 as i32, width, height,
                            0, glow::DEPTH_COMPONENT, glow::UNSIGNED_INT,
                            None);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER,
                                 glow::NEAREST as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER,
                                 glow::NEAREST as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S,
                                 glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T,
                                 glow::CLAMP_TO_EDGE as i32);
            gl.framebuffer_texture_2d(glow::FRAMEBUFFER,
                                      glow::DEPTH_ATTACHMENT,
                                      glow::TEXTURE_2D, Some(id), 0);
            Some(Texture { id, width, height })
        } else {
            let rb = gl.create_renderbuffer().unwrap();
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(rb));
            gl.renderbuffer_storage(glow::RENDERBUFFER,
                                    glow::DEPTH_COMPONENT16, width, height);
            gl.framebuffer_renderbuffer(glow::FRAMEBUFFER,
                                        glow::DEPTH_ATTACHMENT,
                                        glow::RENDERBUFFER, Some(rb));
            None
        };
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        Framebuffer { fb, color, depth }
    }
}

// Route draws to the target (viewport included); None = the screen.
pub fn bind_framebuffer(
    gl: &glow::Context,
    target: Option<&Framebuffer>,
    screen_width: i32,
    screen_height: i32,
) {
    unsafe {
        match target {
            Some(t) => {
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.fb));
                gl.viewport(0, 0, t.color.width, t.color.height);
            }
            None => {
                gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                gl.viewport(0, 0, screen_width, screen_height);
            }
        }
    }
}

// ---- m4 -----------------------------------------------------------------

pub fn perspective(fovy: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fovy / 2.0).tan();
    let nf = 1.0 / (near - far);
    [
        f / aspect, 0.0, 0.0, 0.0,
        0.0, f, 0.0, 0.0,
        0.0, 0.0, (far + near) * nf, -1.0,
        0.0, 0.0, 2.0 * far * near * nf, 0.0,
    ]
}

pub fn translation(x: f32, y: f32, z: f32) -> Mat4 {
    [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        x, y, z, 1.0,
    ]
}

pub fn rotation_y(a: f32) -> Mat4 {
    let (s, c) = a.sin_cos();
    [
        c, 0.0, -s, 0.0,
        0.0, 1.0, 0.0, 0.0,
        s, 0.0, c, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]
}

pub fn multiply(a: &Mat4, b: &Mat4) -> Mat4 { // a . b
    let mut r = [0.0f32; 16];
    for col in 0..4 {
        for row in 0..4 {
            for k in 0..4 {
                r[col * 4 + row] += a[k * 4 + row] * b[col * 4 + k];
            }
        }
    }
    r
}