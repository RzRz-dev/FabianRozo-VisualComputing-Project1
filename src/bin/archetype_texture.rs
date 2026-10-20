// Rust host -- glfw over OpenGL ES 3.0, glow as the GL bindings, vc
// as the thin layer (the crate's lib). The row's ceremony is the
// texture: build the pixels, hand them to vc::create_texture (upload,
// filter, and wrap in one call), and let vc::set_texture route the
// sampler to a unit. No view, no projection -- the quad arrives in
// NDC and the vertex shader passes it through.
//
// run:  cargo run --bin archetype_texture

use glfw::{fail_on_errors, Context};
use glow::HasContext;

// Shader lookup: beside the crate (downloaded files), else at the
// notebook repository's shared location.
fn shader(name: &str) -> String {
    // name is like "archetype_triangle.vert" or "archetype_phong.frag"
    // The archetype directory name matches the filename without the .vert/.frag extension.
    let base = &name[..name.len() - 5]; // strip ".vert" or ".frag" (5 chars)
    std::fs::read_to_string(format!("shaders/{base}/{name}"))
        .or_else(|_| std::fs::read_to_string(format!("../../shaders/{name}")))
        .unwrap()
}

fn main() {
    let mut glfw = glfw::init(fail_on_errors!()).unwrap();
    glfw.window_hint(glfw::WindowHint::ClientApi(
        glfw::ClientApiHint::OpenGlEs,
    ));
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 0));
    let (mut window, _events) = glfw
        .create_window(512, 512, "archetype_texture",
                       glfw::WindowMode::Windowed)
        .unwrap();
    window.make_current();
    let gl = unsafe {
        glow::Context::from_loader_function(|s| {
            window
                .get_proc_address(s)
                .map_or(std::ptr::null(), |p| p as *const _)
        })
    };

    let vert = shader("archetype_texture.vert");
    let frag = shader("archetype_texture.frag");
    let prog = vc::create_program_info(&gl, &vert, &frag);

    // Fullscreen NDC quad; UV (0, 0) at the bottom-left, matching
    // GL's bottom-up texture space.
    let quad = vc::create_buffer_info(&gl, &prog, &[
        vc::Attrib {
            name: "aPosition",
            num_components: 3,
            data: &[-1., -1., 0.,   1., -1., 0.,   1., 1., 0.,
                    -1., -1., 0.,   1.,  1., 0.,  -1., 1., 0.],
        },
        vc::Attrib {
            name: "aTexCoord",
            num_components: 2,
            data: &[0., 0.,   1., 0.,   1., 1.,
                    0., 0.,   1., 1.,   0., 1.],
        },
    ], None);

    // The pattern: a UV gradient (red grows rightward, green upward)
    // under a 32-px checker on blue. Row 0 is v = 0 -- GL's texture
    // space runs bottom-up, and the loop writes it that way, so
    // nothing flips.
    let size = 256usize;
    let mut pixels = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let i = (y * size + x) * 4;
            pixels[i] = x as u8;                                // r
            pixels[i + 1] = y as u8;                            // g
            pixels[i + 2] =
                if ((x >> 5) + (y >> 5)) & 1 == 1 { 255 } else { 0 }; // b
            pixels[i + 3] = 255;
        }
    }
    let tex =
        vc::create_texture(&gl, size as i32, size as i32, Some(&pixels));

    while !window.should_close() {
        let (w, h) = window.get_framebuffer_size();
        unsafe {
            gl.viewport(0, 0, w, h);
            gl.clear_color(20.0 / 255.0, 20.0 / 255.0, 20.0 / 255.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.use_program(Some(prog.program));
        }
        vc::set_texture(&gl, &prog, "uTex", &tex, 0);
        vc::draw_buffer_info(&gl, &quad);

        window.swap_buffers();
        glfw.poll_events();
    }
}