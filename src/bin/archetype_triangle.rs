// Rust host -- glfw over OpenGL ES 3.0, glow as the GL bindings,
// vc.rs as the thin layer. The window hint requests an ES context
// outright, so the shared `#version 300 es` sources compile verbatim.
//
// run:  cargo run --bin archetype_triangle
//       (crate layout: Cargo.toml at the root, the vc layer as the
//       crate's lib at src/vc.rs, one program per row under src/bin/;
//       shaders resolve beside Cargo.toml, else at the notebook
//       repository's shared location)
//
// glfw-rs's init API drifts across releases: 0.62 takes the
// fail_on_errors!() macro, whose expansion needs the function of the
// same name imported at the call site; 0.55 took the plain function.
// Adjust here if the Cargo.toml pin moves.

use glfw::{fail_on_errors, Context};
use glow::HasContext;

// Shader lookup: beside the crate (downloaded files), else at the
// notebook repository's shared location.
fn shader(name: &str) -> String {
    // name is like "archetype_triangle.vert" or "archetype_phong.frag"
    // The archetype directory name matches the filename without the .vert/.frag extension.
    // e.g., "archetype_triangle.vert" -> "archetype_triangle" directory + "archetype_triangle.vert" file
    let base = &name[..name.len() - 5]; // strip ".vert" or ".frag" (5 chars)
    std::fs::read_to_string(format!("shaders/{base}/{name}"))
        .or_else(|_| std::fs::read_to_string(format!("shaders/{name}")))
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
        .create_window(600, 400, "archetype_triangle",
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

    // Shader sources -- the shared files, verbatim.
    let vert = shader("archetype_triangle.vert");
    let frag = shader("archetype_triangle.frag");
    let prog = vc::create_program_info(&gl, &vert, &frag);

    // Attribute buffers, named as the shader declares them.
    let triangle = vc::create_buffer_info(&gl, &prog, &[
        vc::Attrib {
            name: "aPosition",
            num_components: 3,
            data: &[-86.6, -50.0, 0.0,   86.6, -50.0, 0.0,   0.0, 100.0, 0.0],
        },
        vc::Attrib {
            name: "aVertexColor",
            num_components: 4,
            data: &[1.0, 0.0, 0.0, 1.0,   0.0, 1.0, 0.0, 1.0,
                    0.0, 0.0, 1.0, 1.0],
        },
    ], None);

    // Projection built once; view is the camera translation inverted.
    let proj = vc::perspective(std::f32::consts::PI / 3.0, 600.0 / 400.0,
                               0.1, 1000.0);
    let view = vc::translation(0.0, 0.0, -400.0);

    while !window.should_close() {
        let (w, h) = window.get_framebuffer_size();
        unsafe {
            gl.viewport(0, 0, w, h);
            gl.clear_color(20.0 / 255.0, 20.0 / 255.0, 20.0 / 255.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.use_program(Some(prog.program));
        }

        // Model-view = view . R_y(t)
        let mv = vc::multiply(&view, &vc::rotation_y(glfw.get_time() as f32));
        vc::set_uniform(&gl, &prog, "uProjectionMatrix", &proj);
        vc::set_uniform(&gl, &prog, "uModelViewMatrix", &mv);
        vc::draw_buffer_info(&gl, &triangle);

        window.swap_buffers();
        glfw.poll_events();
    }
}