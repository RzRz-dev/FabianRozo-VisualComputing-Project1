// Rust host -- glfw over OpenGL ES 3.0, glow as the GL bindings, vc
// as the thin layer (the crate's lib). New vs render-to-texture: the
// target's DEPTH attachment is a texture -- vc::create_framebuffer's
// with_depth_texture flag -- and the view pass samples it like any
// sampler2D: depth lands in .r. The layer already forces NEAREST on
// it (a non-compare depth texture is not filterable). SPACE toggles
// linearize; the state echoes in the window title.
//
// run:  cargo run --bin archetype_depth

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

// The lighting row's generator, positions and indices; a flat gray
// rides aVertexColor, since color is irrelevant to a depth pass but
// the scene program still consumes the attribute.
fn make_sphere(
    radius: f32,
    lat_bands: usize,
    lon_bands: usize,
) -> (Vec<f32>, Vec<f32>, Vec<u16>) {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    for lat in 0..=lat_bands {
        let phi = lat as f32 * std::f32::consts::PI / lat_bands as f32;
        let (sin_phi, cos_phi) = phi.sin_cos();
        for lon in 0..=lon_bands {
            let theta = lon as f32 * 2.0 * std::f32::consts::PI
                / lon_bands as f32;
            positions.extend([radius * sin_phi * theta.cos(),
                              radius * cos_phi,
                              radius * sin_phi * theta.sin()]);
            colors.extend([0.5, 0.5, 0.5, 1.0]);
        }
    }
    for lat in 0..lat_bands {
        for lon in 0..lon_bands {
            let a = (lat * (lon_bands + 1) + lon) as u16;
            let b = a + lon_bands as u16 + 1;
            indices.extend([a, b, a + 1, b, b + 1, a + 1]);
        }
    }
    (positions, colors, indices)
}

fn main() {
    let mut glfw = glfw::init(fail_on_errors!()).unwrap();
    glfw.window_hint(glfw::WindowHint::ClientApi(
        glfw::ClientApiHint::OpenGlEs,
    ));
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 0));
    let (mut window, _events) = glfw
        .create_window(600, 400, "archetype_depth — linearize on",
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
    let vert = shader("archetype_depth.vert");
    let view_prog = vc::create_program_info(
        &gl,
        &vert,
        &shader("archetype_depth.frag"),
    );

    // Triangle -- the shader-stages one, verbatim.
    let triangle = vc::create_buffer_info(&gl, &scene_prog, &[
        vc::Attrib { name: "aPosition", num_components: 3,
                     data: &[-86.6, -50.0, 0.0,   86.6, -50.0, 0.0,
                             0.0, 100.0, 0.0] },
        vc::Attrib { name: "aVertexColor", num_components: 4,
                     data: &[1.0, 0.0, 0.0, 1.0,   0.0, 1.0, 0.0, 1.0,
                             0.0, 0.0, 1.0, 1.0] },
    ], None);

    let (sph_pos, sph_col, sph_idx) = make_sphere(140.0, 32, 32);
    let sphere = vc::create_buffer_info(&gl, &scene_prog, &[
        vc::Attrib { name: "aPosition", num_components: 3,
                     data: &sph_pos },
        vc::Attrib { name: "aVertexColor", num_components: 4,
                     data: &sph_col },
    ], Some(&sph_idx));

    // Fullscreen quad for the view pass -- the texture row's, verbatim.
    let quad = vc::create_buffer_info(&gl, &view_prog, &[
        vc::Attrib { name: "aPosition", num_components: 3,
                     data: &[-1., -1., 0.,   1., -1., 0.,   1., 1., 0.,
                             -1., -1., 0.,   1.,  1., 0.,  -1., 1., 0.] },
        vc::Attrib { name: "aTexCoord", num_components: 2,
                     data: &[0., 0.,   1., 0.,   1., 1.,
                             0., 0.,   1., 1.,   0., 1.] },
    ], None);

    // The target: color texture + DEPTH TEXTURE (the row's point).
    let target = vc::create_framebuffer(&gl, 600, 400, true);
    let depth_tex = target.depth.as_ref().unwrap();

    // Near sits far out on purpose -- see the view shader's constants,
    // which must match these planes.
    let proj = vc::perspective(std::f32::consts::PI / 3.0, 600.0 / 400.0,
                               100.0, 1000.0);
    let view = vc::translation(0.0, 0.0, -400.0);
    let sphere_mv = vc::translation(0.0, 0.0, -650.0); // world z = -250

    let mut linearize = true;
    let mut prev_space = false;
    while !window.should_close() {
        // SPACE toggles linearize (edge-triggered).
        let space = window.get_key(glfw::Key::Space) == glfw::Action::Press;
        if space && !prev_space {
            linearize = !linearize;
            window.set_title(&format!(
                "archetype_depth — linearize {}",
                if linearize { "on" } else { "off" }
            ));
        }
        prev_space = space;

        let (w, h) = window.get_framebuffer_size();

        // Scene phase: triangle + sphere into the target, depth test
        // on so the map records occlusion.
        vc::bind_framebuffer(&gl, Some(&target), w, h);
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

        vc::set_uniform(&gl, &scene_prog, "uModelViewMatrix", &sphere_mv);
        vc::draw_buffer_info(&gl, &sphere); // indexed

        // View phase: the depth texture to the screen, as grayscale.
        vc::bind_framebuffer(&gl, None, w, h);
        unsafe {
            gl.disable(glow::DEPTH_TEST);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.use_program(Some(view_prog.program));
        }
        vc::set_texture(&gl, &view_prog, "uDepth", depth_tex, 0);
        vc::set_uniform_i32(&gl, &view_prog, "uLinearize",
                            if linearize { 1 } else { 0 });
        vc::draw_buffer_info(&gl, &quad);

        window.swap_buffers();
        glfw.poll_events();
    }
}