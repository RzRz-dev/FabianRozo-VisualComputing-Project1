// Rust host -- glfw over OpenGL ES 3.0, glow as the GL bindings, vc
// as the thin layer (the crate's lib). New vs the earlier rows: a
// second per-vertex attribute (aNormal), an indexed mesh (the index
// list rides create_buffer_info and draw_buffer_info switches to
// draw_elements), a mat3 uniform (the normal matrix), and the depth
// test. The sphere is hand-rolled -- the same lat/lon loop every host
// carries. UP/DOWN keys tune the shininess; the value echoes in the
// window title.
//
// run:  cargo run --bin archetype_phong

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

// UV sphere: latitude x longitude grid. Vertex at
// (sin(phi)cos(theta), cos(phi), sin(phi)sin(theta)) * r; the normal
// is the unit vertex, since the sphere is centred at the origin.
fn make_sphere(
    radius: f32,
    lat_bands: usize,
    lon_bands: usize,
) -> (Vec<f32>, Vec<f32>, Vec<u16>) {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    for lat in 0..=lat_bands {
        let phi = lat as f32 * std::f32::consts::PI / lat_bands as f32;
        let (sin_phi, cos_phi) = phi.sin_cos();
        for lon in 0..=lon_bands {
            let theta = lon as f32 * 2.0 * std::f32::consts::PI
                / lon_bands as f32;
            let x = sin_phi * theta.cos();
            let y = cos_phi;
            let z = sin_phi * theta.sin();
            positions.extend([radius * x, radius * y, radius * z]);
            normals.extend([x, y, z]);
        }
    }
    for lat in 0..lat_bands {
        for lon in 0..lon_bands {
            let a = (lat * (lon_bands + 1) + lon) as u16;
            let b = a + lon_bands as u16 + 1;
            indices.extend([a, b, a + 1, b, b + 1, a + 1]);
        }
    }
    (positions, normals, indices)
}

fn main() {
    let mut glfw = glfw::init(fail_on_errors!()).unwrap();
    glfw.window_hint(glfw::WindowHint::ClientApi(
        glfw::ClientApiHint::OpenGlEs,
    ));
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 0));
    let (mut window, _events) = glfw
        .create_window(600, 400, "archetype_phong",
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
    unsafe { gl.enable(glow::DEPTH_TEST) };

    let vert = shader("archetype_phong.vert");
    let frag = shader("archetype_phong.frag");
    let prog = vc::create_program_info(&gl, &vert, &frag);

    let (positions, normals, indices) = make_sphere(100.0, 32, 32);
    let mesh = vc::create_buffer_info(&gl, &prog, &[
        vc::Attrib { name: "aPosition", num_components: 3,
                     data: &positions },
        vc::Attrib { name: "aNormal", num_components: 3, data: &normals },
    ], Some(&indices));

    // Sphere at the origin, camera at z = 350: the model-view is a
    // pure translation, so the normal matrix (inverse-transpose of
    // its upper 3x3) is the identity -- pushed explicitly all the
    // same.
    let proj = vc::perspective(std::f32::consts::PI / 3.0, 600.0 / 400.0,
                               0.1, 1000.0);
    let mv = vc::translation(0.0, 0.0, -350.0);
    let normal_mat: vc::Mat3 =
        [1., 0., 0.,  0., 1., 0.,  0., 0., 1.];

    let mut spec_power = 32.0f32;
    while !window.should_close() {
        // UP/DOWN tune the shininess.
        let up = window.get_key(glfw::Key::Up) == glfw::Action::Press;
        let down = window.get_key(glfw::Key::Down) == glfw::Action::Press;
        if up || down {
            spec_power += if up { 1.0 } else { -1.0 };
            spec_power = spec_power.clamp(1.0, 128.0);
            window.set_title(&format!("archetype_phong — shininess {}",
                                      spec_power as i32));
        }

        let (w, h) = window.get_framebuffer_size();
        unsafe {
            gl.viewport(0, 0, w, h);
            gl.clear_color(20.0 / 255.0, 20.0 / 255.0, 20.0 / 255.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
            gl.use_program(Some(prog.program));
        }

        // Light orbits in world space; the camera is a pure
        // translation, so eye-space light = world light - (0, 0, 350).
        let a = glfw.get_time() as f32;
        let light_eye = [a.cos() * 200.0, 80.0, a.sin() * 200.0 - 350.0];

        vc::set_uniform(&gl, &prog, "uProjectionMatrix", &proj);
        vc::set_uniform(&gl, &prog, "uModelViewMatrix", &mv);
        vc::set_uniform_mat3(&gl, &prog, "uNormalMatrix", &normal_mat);
        vc::set_uniform_vec3(&gl, &prog, "uLightPosition", &light_eye);
        vc::set_uniform_vec3(&gl, &prog, "uColor", &[0.3, 0.55, 0.9]);
        vc::set_uniform_f32(&gl, &prog, "uAmbient", 0.15);
        vc::set_uniform_f32(&gl, &prog, "uSpecPower", spec_power);
        vc::draw_buffer_info(&gl, &mesh);   // indexed: draw_elements

        window.swap_buffers();
        glfw.poll_events();
    }
}