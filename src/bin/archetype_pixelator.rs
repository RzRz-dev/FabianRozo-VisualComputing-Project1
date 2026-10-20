// Rust host -- glfw over OpenGL ES 3.0, glow as the GL bindings, vc
// as the thin layer (the crate's lib). New vs the earlier rows: two
// programs, two geometries, and an offscreen render target.
// vc::create_framebuffer builds the FBO (color texture + depth
// renderbuffer); vc::bind_framebuffer routes the pass and sets the
// viewport -- Some(&target) for the scene phase, None for the screen
// phase. UP/DOWN keys tune the block count; the value echoes in the
// window title.
//
// run:  cargo run --bin archetype_pixelator

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
        .create_window(600, 400, "archetype_pixelator",
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

    let scene_prog = vc::create_program_info(
        &gl,
        &shader("archetype_triangle.vert"),
        &shader("archetype_triangle.frag"),
    );
    let post_prog = vc::create_program_info(
        &gl,
        &shader("archetype_pixelator.vert"),
        &shader("archetype_pixelator.frag"),
    );

    // Scene geometry: the first row's triangle, verbatim.
    let triangle = vc::create_buffer_info(&gl, &scene_prog, &[
        vc::Attrib { name: "aPosition", num_components: 3,
                     data: &[-86.6, -50.0, 0.0,   86.6, -50.0, 0.0,
                             0.0, 100.0, 0.0] },
        vc::Attrib { name: "aVertexColor", num_components: 4,
                     data: &[1.0, 0.0, 0.0, 1.0,   0.0, 1.0, 0.0, 1.0,
                             0.0, 0.0, 1.0, 1.0] },
    ], None);

    // Post geometry: the texture row's fullscreen NDC quad, verbatim.
    let quad = vc::create_buffer_info(&gl, &post_prog, &[
        vc::Attrib { name: "aPosition", num_components: 3,
                     data: &[-1., -1., 0.,   1., -1., 0.,   1., 1., 0.,
                             -1., -1., 0.,   1.,  1., 0.,  -1., 1., 0.] },
        vc::Attrib { name: "aTexCoord", num_components: 2,
                     data: &[0., 0.,   1., 0.,   1., 1.,
                             0., 0.,   1., 1.,   0., 1.] },
    ], None);

    // The render target: color texture + depth renderbuffer.
    let target = vc::create_framebuffer(&gl, 600, 400, false);

    let proj = vc::perspective(std::f32::consts::PI / 3.0, 600.0 / 400.0,
                               0.1, 1000.0);
    let view = vc::translation(0.0, 0.0, -400.0);

    let mut blocks = 50.0f32;
    while !window.should_close() {
        // UP/DOWN tune the block count.
        let up = window.get_key(glfw::Key::Up) == glfw::Action::Press;
        let down = window.get_key(glfw::Key::Down) == glfw::Action::Press;
        if up || down {
            blocks += if up { 2.0 } else { -2.0 };
            blocks = blocks.clamp(8.0, 60.0);
            window.set_title(&format!("archetype_pixelator — blocks {}",
                                      blocks as i32));
        }

        let (w, h) = window.get_framebuffer_size();

        // Scene phase: the rotating triangle, into the target.
        vc::bind_framebuffer(&gl, Some(&target), w, h);
        unsafe {
            gl.clear_color(20.0 / 255.0, 20.0 / 255.0, 20.0 / 255.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
            gl.use_program(Some(scene_prog.program));
        }
        let mv = vc::multiply(&view, &vc::rotation_y(glfw.get_time() as f32));
        vc::set_uniform(&gl, &scene_prog, "uProjectionMatrix", &proj);
        vc::set_uniform(&gl, &scene_prog, "uModelViewMatrix", &mv);
        vc::draw_buffer_info(&gl, &triangle);

        // Post phase: resample the target's color texture to the
        // screen, UVs snapped to block centers.
        vc::bind_framebuffer(&gl, None, w, h);
        unsafe {
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.use_program(Some(post_prog.program));
        }
        vc::set_texture(&gl, &post_prog, "uScene", &target.color, 0);
        vc::set_uniform_f32(&gl, &post_prog, "uBlocks", blocks);
        vc::draw_buffer_info(&gl, &quad);

        window.swap_buffers();
        glfw.poll_events();
    }
}