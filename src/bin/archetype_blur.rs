// Rust host -- glfw over OpenGL ES 3.0, glow as the GL bindings, vc
// as the thin layer (the crate's lib). Multipass: two targets,
// ping-ponged. One blur_pass closure is the whole pattern -- bind the
// destination (Some(&target), or None for the screen), point uScene
// at the source texture, set uDir, draw the quad -- run twice with
// the buffers swapped: A to B horizontally, B to screen vertically.
// UP/DOWN keys tune the radius; the value echoes in the window title.
//
// run:  cargo run --bin archetype_blur

use glfw::{fail_on_errors, Context};
use glow::HasContext;

// Shader lookup: beside the crate (downloaded files), else at the
// notebook repository's shared location.
fn shader(name: &str) -> String {
    // name is like "archetype_triangle.vert" or "archetype_phong.frag"
    // The archetype directory name matches the filename without the .vert/.frag extension.
    let base = &name[..name.len() - 5]; // strip ".vert" or ".frag" (5 chars)
    std::fs::read_to_string(format!("shaders/{base}/{name}"))
        .or_else(|_| std::fs::read_to_string(name))
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
        .create_window(600, 400, "archetype_blur — radius 2.0",
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
    let blur_prog = vc::create_program_info(
        &gl,
        &shader("archetype_blur.vert"),
        &shader("archetype_blur.frag"),
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
    let quad = vc::create_buffer_info(&gl, &blur_prog, &[
        vc::Attrib { name: "aPosition", num_components: 3,
                     data: &[-1., -1., 0.,   1., -1., 0.,   1., 1., 0.,
                             -1., -1., 0.,   1.,  1., 0.,  -1., 1., 0.] },
        vc::Attrib { name: "aTexCoord", num_components: 2,
                     data: &[0., 0.,   1., 0.,   1., 1.,
                             0., 0.,   1., 1.,   0., 1.] },
    ], None);

    // Two targets: A carries depth for the scene; B is color-only.
    let fbo_a = vc::create_framebuffer(&gl, 600, 400, false);
    let fbo_b = vc::create_framebuffer(&gl, 600, 400, false);

    let proj = vc::perspective(std::f32::consts::PI / 3.0, 600.0 / 400.0,
                               0.1, 1000.0);
    let view = vc::translation(0.0, 0.0, -400.0);

    // One separable pass: src texture -> dst target (None = screen).
    let blur_pass = |src: &vc::Texture, dst: Option<&vc::Framebuffer>,
                     dir: [f32; 2], w: i32, h: i32| {
        vc::bind_framebuffer(&gl, dst, w, h);
        unsafe {
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.use_program(Some(blur_prog.program));
        }
        vc::set_texture(&gl, &blur_prog, "uScene", src, 0);
        vc::set_uniform_vec2(&gl, &blur_prog, "uDir", &dir);
        vc::draw_buffer_info(&gl, &quad);
    };

    let mut radius = 2.0f32;
    let mut prev_up = false;
    let mut prev_down = false;
    while !window.should_close() {
        // UP/DOWN tune the radius (edge-triggered, 0.5 steps).
        let up = window.get_key(glfw::Key::Up) == glfw::Action::Press;
        let down = window.get_key(glfw::Key::Down) == glfw::Action::Press;
        if (up && !prev_up) || (down && !prev_down) {
            radius += if up { 0.5 } else { -0.5 };
            radius = radius.clamp(0.0, 6.0);
            window.set_title(&format!("archetype_blur — radius {radius:.1}"));
        }
        prev_up = up;
        prev_down = down;

        let (w, h) = window.get_framebuffer_size();

        // Scene phase: the rotating triangle, into A.
        vc::bind_framebuffer(&gl, Some(&fbo_a), w, h);
        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.clear_color(20.0 / 255.0, 20.0 / 255.0, 20.0 / 255.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
            gl.use_program(Some(scene_prog.program));
        }
        let mv = vc::multiply(&view, &vc::rotation_y(glfw.get_time() as f32));
        vc::set_uniform(&gl, &scene_prog, "uProjectionMatrix", &proj);
        vc::set_uniform(&gl, &scene_prog, "uModelViewMatrix", &mv);
        vc::draw_buffer_info(&gl, &triangle);

        // Ping-pong: horizontal (A -> B), then vertical (B -> screen).
        unsafe { gl.disable(glow::DEPTH_TEST) };
        blur_pass(&fbo_a.color, Some(&fbo_b), [radius / 600.0, 0.0], w, h);
        blur_pass(&fbo_b.color, None, [0.0, radius / 400.0], w, h);

        window.swap_buffers();
        glfw.poll_events();
    }
}