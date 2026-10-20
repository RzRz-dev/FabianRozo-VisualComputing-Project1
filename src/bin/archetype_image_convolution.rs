// Rust host -- glfw over OpenGL ES 3.0, glow as the GL bindings, vc as the thin layer.
// Fullscreen convolution pass over images AND videos. Every image/video found in
// the images folder is selectable at runtime; the kernel is picked with the keyboard.
// Video decoding uses ffmpeg-next.
//
// run:  cargo run --bin archetype_image_convolution
//       cargo run --bin archetype_image_convolution -- cat.png     (start on this file)
//       cargo run --bin archetype_image_convolution -- path/to/dir (use this folder)
//
// Files are auto-discovered in ./images and ../../images (plus the folder you
// pass, or the folder of the file you pass).
//   images: png, jpg, jpeg, bmp, gif, tiff
//   videos: mp4, mov, mkv, webm, avi, m4v
//
// controls:
//   Up / Down      next / previous image or video
//   Left / Right   previous / next kernel
//   1..9           pick kernel 1-9 directly (Left/Right reaches all 12)
//   Space          toggle original (no filter)
//   P              pause / resume video
//   [ / ]          lower / raise bias (hold)
//   R              reset bias
//   Esc            quit
//
use ffmpeg_next as ffmpeg;
use ffmpeg::format::Pixel;
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context as Scaler, flag::Flags};
use glfw::{fail_on_errors, Action, Context, Key};
use glow::HasContext;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const KERNEL_NAMES: [&str; 12] = [
    "Ridge (Sobel X)", "Emboss", "Sharpen", "Box blur", "Identity",
    "Gaussian blur", "Sobel Y", "Laplacian", "Outline", "Sharpen strong",
    "Motion blur (diagonal)", "Sobel magnitude",
];
// Base bias per kernel. Kernels centred on 0 need 0.5 so flat areas show as mid-gray.
const KERNEL_BIAS: [f32; 12] = [
    0.5, 0.5, 0.0, 0.0, 0.0,
    0.0, 0.5, 0.5, 0.0, 0.0,
    0.0, 0.0,
];
const IMAGE_EXTS: [&str; 6] = ["png", "jpg", "jpeg", "bmp", "gif", "tiff"];
const VIDEO_EXTS: [&str; 6] = ["mp4", "mov", "mkv", "webm", "avi", "m4v"];

// Shader lookup: beside the crate, else at the notebook repository's shared location.
fn shader(name: &str) -> String {
    let base = &name[..name.len() - 5]; // strip ".vert" or ".frag"
    std::fs::read_to_string(format!("shaders/{base}/{name}"))
        .or_else(|_| std::fs::read_to_string(name))
        .or_else(|_| std::fs::read_to_string(format!("../../shaders/{name}")))
        .unwrap_or_else(|_| panic!("could not find shader file: {name}"))
}

// ================================================================ discovery

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Image,
    Video,
}

struct Item {
    path: PathBuf,
    kind: Kind,
}

fn ext_of(p: &Path) -> String {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default()
}

fn kind_of(p: &Path) -> Option<Kind> {
    let e = ext_of(p);
    if IMAGE_EXTS.contains(&e.as_str()) {
        Some(Kind::Image)
    } else if VIDEO_EXTS.contains(&e.as_str()) {
        Some(Kind::Video)
    } else {
        None
    }
}

fn scan_dir(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && kind_of(&p).is_some() {
                out.push(p);
            }
        }
    }
}

// Returns every image/video found, sorted, plus the index to start on.
fn discover(arg: Option<&str>) -> (Vec<Item>, usize) {
    let mut dirs: Vec<PathBuf> = vec![];
    let mut start_file: Option<PathBuf> = None;
    let mut start_name: Option<String> = None;

    if let Some(a) = arg {
        let p = PathBuf::from(a);
        if p.is_dir() {
            dirs.push(p);
        } else if p.is_file() {
            let parent = p.parent().map(|d| d.to_path_buf()).unwrap_or_default();
            dirs.push(if parent.as_os_str().is_empty() { PathBuf::from(".") } else { parent });
            start_file = Some(p);
        } else {
            start_name = Some(a.to_lowercase()); // just a name: match it against what we find
        }
    }
    dirs.push(PathBuf::from("images"));
    dirs.push(PathBuf::from("../../images"));

    let mut found = vec![];
    for d in &dirs {
        scan_dir(d, &mut found);
    }
    let mut paths: Vec<PathBuf> = found.into_iter().filter_map(|p| p.canonicalize().ok()).collect();
    paths.sort();
    paths.dedup();

    let mut start = 0;
    if let Some(f) = start_file {
        if let Ok(c) = f.canonicalize() {
            match paths.iter().position(|p| *p == c) {
                Some(i) => start = i,
                None => {
                    paths.push(c); // explicit file with an unusual extension: still allow it
                    start = paths.len() - 1;
                }
            }
        }
    } else if let Some(name) = start_name {
        let hit = paths.iter().position(|p| {
            let file = p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            file == name || stem == name
        });
        match hit {
            Some(i) => start = i,
            None => eprintln!("warning: no file matching '{name}', starting on the first one"),
        }
    }

    if paths.is_empty() {
        panic!(
            "no images or videos found. Searched: {}\nPut some files in ./images or ../../images, or pass a folder.",
            dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>().join(", ")
        );
    }
    let items = paths
        .into_iter()
        .map(|path| {
            let kind = kind_of(&path).unwrap_or(Kind::Image);
            Item { path, kind }
        })
        .collect();
    (items, start)
}

// ================================================================ textures

fn new_texture(gl: &glow::Context, width: i32, height: i32, pixels: Option<&[u8]>) -> vc::Texture {
    let id = unsafe { gl.create_texture().unwrap() };
    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(id));
        gl.tex_image_2d(
            glow::TEXTURE_2D, 0, glow::RGBA as i32, width, height, 0,
            glow::RGBA, glow::UNSIGNED_BYTE, pixels,
        );
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
    }
    vc::Texture { id, width, height }
}

// Replace the contents of an existing texture with a new video frame (same size).
fn upload_frame(gl: &glow::Context, tex: &vc::Texture, rgba: &[u8]) {
    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(tex.id));
        gl.tex_image_2d(
            glow::TEXTURE_2D, 0, glow::RGBA as i32, tex.width, tex.height, 0,
            glow::RGBA, glow::UNSIGNED_BYTE, Some(rgba),
        );
    }
}

fn try_load_image(gl: &glow::Context, path: &Path) -> Option<vc::Texture> {
    let img = match image::open(path) {
        Ok(i) => i.flipv().into_rgba8(), // flip: OpenGL row 0 is the bottom
        Err(e) => {
            eprintln!("skipping {}: {e}", path.display());
            return None;
        }
    };
    let (w, h) = img.dimensions();
    let pixels: Vec<u8> = img.into_raw();
    Some(new_texture(gl, w as i32, h as i32, Some(&pixels)))
}

// ================================================================ video (ffmpeg-next)

// Thin wrapper over demuxer + decoder + RGBA scaler. Produces one frame at a time.
struct VideoDecoder {
    ictx: ffmpeg::format::context::Input,
    decoder: ffmpeg::decoder::Video,
    scaler: Scaler,
    stream_index: usize,
    time_base: f64,
    frame_dur: f64,
    last_pts: f64,
    eof: bool,
    width: usize,
    height: usize,
}

impl VideoDecoder {
    fn open(path: &Path) -> Result<Self, String> {
        let ictx = ffmpeg::format::input(&path).map_err(|e| e.to_string())?;
        let (stream_index, time_base, frame_dur, decoder) = {
            let stream = ictx
                .streams()
                .best(Type::Video)
                .ok_or_else(|| "no video stream".to_string())?;
            let tb = stream.time_base();
            let time_base = tb.numerator() as f64 / tb.denominator() as f64;
            let fr = stream.avg_frame_rate();
            let frame_dur = if fr.numerator() > 0 && fr.denominator() > 0 {
                fr.denominator() as f64 / fr.numerator() as f64
            } else {
                1.0 / 30.0
            };
            let ctx = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
                .map_err(|e| e.to_string())?;
            let decoder = ctx.decoder().video().map_err(|e| e.to_string())?;
            (stream.index(), time_base, frame_dur, decoder)
        };
        let (w, h) = (decoder.width(), decoder.height());
        if w == 0 || h == 0 {
            return Err("video has zero size".to_string());
        }
        let scaler = Scaler::get(decoder.format(), w, h, Pixel::RGBA, w, h, Flags::BILINEAR)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            ictx, decoder, scaler, stream_index, time_base, frame_dur,
            last_pts: 0.0, eof: false, width: w as usize, height: h as usize,
        })
    }

    // Decode the next frame into `out` (RGBA, rows flipped so row 0 = bottom, like the
    // image path). Returns its presentation time in seconds, or None at end of file.
    fn decode_into(&mut self, out: &mut Vec<u8>) -> Option<f64> {
        let mut decoded = ffmpeg::frame::Video::empty();
        loop {
            if self.decoder.receive_frame(&mut decoded).is_ok() {
                let mut rgba = ffmpeg::frame::Video::empty();
                self.scaler.run(&decoded, &mut rgba).ok()?;

                let (w, h) = (self.width, self.height);
                let stride: usize = rgba.stride(0);
                let data: &[u8] = rgba.data(0);
                let row: usize = w * 4;
                out.resize(row * h, 0);
                for y in 0..h {
                    let start: usize = y * stride;
                    let src: &[u8] = &data[start..start + row];
                    let dst_start: usize = (h - 1 - y) * row;
                    out[dst_start..dst_start + row].copy_from_slice(src);
                }

                let pts = match decoded.timestamp().or(decoded.pts()) {
                    Some(t) => t as f64 * self.time_base,
                    None => self.last_pts + self.frame_dur,
                };
                self.last_pts = pts;
                return Some(pts);
            }
            if self.eof {
                return None;
            }
            // decoder wants more input
            let next = self.ictx.packets().next().map(|(s, p)| (s.index(), p));
            match next {
                Some((idx, packet)) => {
                    if idx == self.stream_index {
                        let _ = self.decoder.send_packet(&packet);
                    }
                }
                None => {
                    let _ = self.decoder.send_eof();
                    self.eof = true;
                }
            }
        }
    }
}

// Real-time playback: decodes ahead by one frame and releases it when its time comes.
struct VideoPlayer {
    path: PathBuf,
    dec: VideoDecoder,
    width: i32,
    height: i32,
    current: Vec<u8>, // frame to display
    pending: Vec<u8>, // next decoded frame, waiting for its time
    pending_pts: Option<f64>,
    last_shown: f64,
    base_pts: Option<f64>,
    clock: Instant,
    paused_at: Option<Instant>,
}

impl VideoPlayer {
    fn open(path: &Path) -> Result<Self, String> {
        let dec = VideoDecoder::open(path)?;
        let (width, height) = (dec.width as i32, dec.height as i32);
        Ok(Self {
            path: path.to_path_buf(),
            dec, width, height,
            current: vec![],
            pending: vec![],
            pending_pts: None,
            last_shown: 0.0,
            base_pts: None,
            clock: Instant::now(),
            paused_at: None,
        })
    }

    fn set_paused(&mut self, paused: bool) {
        match (paused, self.paused_at) {
            (true, None) => self.paused_at = Some(Instant::now()),
            (false, Some(t)) => {
                self.clock += t.elapsed(); // don't count the pause as playback time
                self.paused_at = None;
            }
            _ => {}
        }
    }

    // Returns true when `current` holds a new frame that should be uploaded.
    fn update(&mut self) -> bool {
        if self.paused_at.is_some() {
            return false;
        }
        let mut changed = false;
        for _ in 0..8 {
            if self.pending_pts.is_none() {
                let mut buf = std::mem::take(&mut self.pending);
                let got = self.dec.decode_into(&mut buf);
                self.pending = buf;
                match got {
                    Some(pts) => {
                        let base = *self.base_pts.get_or_insert(pts);
                        self.pending_pts = Some(pts - base);
                    }
                    None => {
                        // end of file: loop from the start
                        if let Ok(d) = VideoDecoder::open(&self.path) {
                            self.dec = d;
                            self.base_pts = None;
                            self.clock = Instant::now();
                        }
                        return changed;
                    }
                }
            }
            let p = self.pending_pts.unwrap();
            if p <= self.clock.elapsed().as_secs_f64() {
                std::mem::swap(&mut self.current, &mut self.pending);
                self.pending_pts = None;
                self.last_shown = p;
                changed = true;
            } else {
                return changed;
            }
        }
        // We are far behind (slow decode or a stall): resync the clock instead of racing.
        let back = Duration::from_secs_f64(self.last_shown.max(0.0));
        self.clock = Instant::now().checked_sub(back).unwrap_or_else(Instant::now);
        changed
    }
}

// ================================================================ library of media

struct Library {
    items: Vec<Item>,
    textures: Vec<Option<vc::Texture>>,
    players: Vec<Option<VideoPlayer>>,
    failed: Vec<bool>,
}

impl Library {
    fn new(items: Vec<Item>) -> Self {
        let n = items.len();
        Self {
            items,
            textures: (0..n).map(|_| None).collect(),
            players: (0..n).map(|_| None).collect(),
            failed: vec![false; n],
        }
    }

    // Make item `i` ready to display. Images load once; videos get a fresh player
    // (playing from the start) every time they are selected.
    fn prepare(&mut self, gl: &glow::Context, i: usize) -> bool {
        let path = self.items[i].path.clone();
        match self.items[i].kind {
            Kind::Image => {
                if self.textures[i].is_none() {
                    match try_load_image(gl, &path) {
                        Some(t) => self.textures[i] = Some(t),
                        None => {
                            self.failed[i] = true;
                            return false;
                        }
                    }
                }
                true
            }
            Kind::Video => match VideoPlayer::open(&path) {
                Ok(mut p) => {
                    if self.textures[i].is_none() {
                        self.textures[i] = Some(new_texture(gl, p.width, p.height, None));
                    }
                    if p.update() {
                        upload_frame(gl, self.textures[i].as_ref().unwrap(), &p.current);
                    }
                    self.players[i] = Some(p);
                    true
                }
                Err(e) => {
                    eprintln!("skipping {}: {e}", path.display());
                    self.failed[i] = true;
                    false
                }
            },
        }
    }

    // Starting at `start`, walk in direction `dir` until something loads.
    fn activate(&mut self, gl: &glow::Context, start: usize, dir: i32) -> usize {
        let n = self.items.len() as i32;
        let mut idx = start as i32;
        for _ in 0..n {
            let i = idx as usize;
            if !self.failed[i] && self.prepare(gl, i) {
                return i;
            }
            idx = (idx + dir + n) % n;
        }
        panic!("none of the files could be opened");
    }

    // Stop decoding a video we are leaving (frees the decoder).
    fn deactivate(&mut self, i: usize) {
        self.players[i] = None;
    }
}

// ================================================================ misc

fn title(pos: usize, total: usize, item: &Item, kernel: i32, original: bool, bias: f32, paused: bool) -> String {
    let name = item.path.file_name().and_then(|s| s.to_str()).unwrap_or("?");
    let mode = if original { "ORIGINAL" } else { KERNEL_NAMES[kernel as usize] };
    let kind = match (item.kind, paused) {
        (Kind::Video, true) => " [video, paused]",
        (Kind::Video, false) => " [video]",
        (Kind::Image, _) => "",
    };
    format!(
        "[{}/{}] {}{} | {} | bias {:+.2} | Up/Down file, Left/Right kernel, Space original, P pause",
        pos + 1, total, name, kind, mode, bias
    )
}

fn main() {
    ffmpeg::init().expect("failed to initialise ffmpeg");
    ffmpeg::util::log::set_level(ffmpeg::util::log::Level::Error);

    // ---- find every image/video ----
    let arg = std::env::args().nth(1);
    let (items, start) = discover(arg.as_deref());
    println!("found {} file(s):", items.len());
    for (i, it) in items.iter().enumerate() {
        let k = if it.kind == Kind::Video { "video" } else { "image" };
        println!("  {i}: {} ({k})", it.path.file_name().and_then(|s| s.to_str()).unwrap_or("?"));
    }

    // ---- window ----
    let mut glfw = glfw::init(fail_on_errors).unwrap();
    glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::OpenGlEs));
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 0));
    let (mut window, _events) = glfw
        .create_window(1000, 700, "archetype_image_convolution", glfw::WindowMode::Windowed)
        .expect("failed to create window");
    window.make_current();
    glfw.set_swap_interval(glfw::SwapInterval::Sync(1)); // vsync
    let gl = unsafe {
        glow::Context::from_loader_function(|s| {
            window
                .get_proc_address(s)
                .map_or(std::ptr::null(), |p| p as *const _)
        })
    };

    let conv_prog = vc::create_program_info(
        &gl,
        &shader("archetype_convolution.vert"),
        &shader("archetype_convolution.frag"),
    );

    // Fullscreen quad (NDC) with texture coordinates
    let quad = vc::create_buffer_info(&gl, &conv_prog, &[
        vc::Attrib { name: "aPosition", num_components: 3,
                     data: &[-1., -1., 0.,   1., -1., 0.,   1., 1., 0.,
                              -1., -1., 0.,   1.,  1., 0.,  -1., 1., 0.] },
        vc::Attrib { name: "aTexCoord", num_components: 2,
                     data: &[0., 0.,   1., 0.,   1., 1.,
                              0., 0.,   1., 1.,   0., 1.] },
    ], None);

    // ---- state ----
    let mut lib = Library::new(items);
    let n_items = lib.items.len();
    let mut image_idx = lib.activate(&gl, start, 1);

    let n_kernels = KERNEL_NAMES.len() as i32;
    let mut kernel_idx = 0i32; // 0=Ridge 1=Emboss 2=Sharpen 3=Blur 4=Identity
    let mut show_original = false;
    let mut paused = false;
    let mut bias_offset: f32 = 0.0;

    // one-shot keys: Left, Right, Up, Down, Space, 1..5, R, P
    let keys = [
        Key::Left, Key::Right, Key::Up, Key::Down, Key::Space,
        Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5,
        Key::R, Key::P,
        Key::Num6, Key::Num7, Key::Num8, Key::Num9,
    ];
    let mut was_down = [false; 16];
    let mut dirty = true;

    while !window.should_close() {
        let mut pressed = [false; 16];
        for (i, k) in keys.iter().enumerate() {
            let down = window.get_key(*k) == Action::Press;
            pressed[i] = down && !was_down[i];
            was_down[i] = down;
        }

        // kernel selector
        if pressed[0] { kernel_idx = (kernel_idx + n_kernels - 1) % n_kernels; }
        if pressed[1] { kernel_idx = (kernel_idx + 1) % n_kernels; }
        for i in 0..5 {
            if pressed[5 + i] { kernel_idx = i as i32; }       // keys 1..5
        }
        for i in 0..4 {
            if pressed[12 + i] { kernel_idx = 5 + i as i32; }  // keys 6..9
        }
        // media selector
        if pressed[2] {
            lib.deactivate(image_idx);
            let next = (image_idx + 1) % n_items;
            image_idx = lib.activate(&gl, next, 1);
            paused = false;
        }
        if pressed[3] {
            lib.deactivate(image_idx);
            let prev = (image_idx + n_items - 1) % n_items;
            image_idx = lib.activate(&gl, prev, -1);
            paused = false;
        }
        if pressed[4] { show_original = !show_original; }
        if pressed[10] { bias_offset = 0.0; }
        if pressed[11] {
            paused = !paused;
            if let Some(p) = lib.players[image_idx].as_mut() {
                p.set_paused(paused);
            }
        }

        // held keys: bias
        if window.get_key(Key::LeftBracket) == Action::Press { bias_offset -= 0.005; dirty = true; }
        if window.get_key(Key::RightBracket) == Action::Press { bias_offset += 0.005; dirty = true; }
        bias_offset = bias_offset.clamp(-1.0, 1.0);
        if window.get_key(Key::Escape) == Action::Press { window.set_should_close(true); }

        let base_bias = KERNEL_BIAS[kernel_idx as usize];
        let bias = base_bias + bias_offset;

        if dirty || pressed.iter().any(|p| *p) {
            window.set_title(&title(
                image_idx, n_items, &lib.items[image_idx],
                kernel_idx, show_original, bias, paused,
            ));
            dirty = false;
        }

        // advance the video, if the current item is one
        if let (Some(p), Some(t)) = (lib.players[image_idx].as_mut(), lib.textures[image_idx].as_ref()) {
            if p.update() {
                upload_frame(&gl, t, &p.current);
            }
        }

        let tex = lib.textures[image_idx].as_ref().unwrap();
        let (w, h) = window.get_framebuffer_size();

        vc::bind_framebuffer(&gl, None, w, h);
        unsafe {
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            // letterbox: keep the source aspect ratio inside the window
            let s = (w as f32 / tex.width as f32).min(h as f32 / tex.height as f32);
            let vw = ((tex.width as f32 * s) as i32).max(1);
            let vh = ((tex.height as f32 * s) as i32).max(1);
            gl.viewport((w - vw) / 2, (h - vh) / 2, vw, vh);

            gl.use_program(Some(conv_prog.program));
        }

        vc::set_texture(&gl, &conv_prog, "uSource", tex, 0);
        // one texel of the SOURCE, not one pixel of the window
        vc::set_uniform_vec2(&gl, &conv_prog, "uTexelSize",
            &[1.0 / tex.width as f32, 1.0 / tex.height as f32]);
        vc::set_uniform_i32(&gl, &conv_prog, "uKernel", kernel_idx);
        vc::set_uniform_f32(&gl, &conv_prog, "uBias", bias);
        vc::set_uniform_i32(&gl, &conv_prog, "uOriginal", show_original as i32);

        vc::draw_buffer_info(&gl, &quad);

        window.swap_buffers();
        glfw.poll_events();
    }
}