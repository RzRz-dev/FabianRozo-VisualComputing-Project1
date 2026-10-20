# FabianRozo-VisualComputing-Project1

# Image & Video Convolution Viewer

A Rust program that applies 3x3 convolution kernels to images and videos in real time on the GPU. It uses GLFW for the window, OpenGL ES 3.0 through `glow`, the `image` crate for pictures and `ffmpeg-next` for video. Every image and video found in the images folder can be selected while the program runs, and the kernel is chosen from the keyboard.

## Project layout

```
archetypes/
├── Cargo.toml
├── images/                  <- put your images and videos here
├── shaders/
│   └── archetype_convolution/
│       ├── archetype_convolution.vert
│       └── archetype_convolution.frag
└── src/
    ├── main.rs
    ├── vc.rs                <- helper module (built as the `vc` library)
    └── bin/
        └── archetype_image_convolution.rs
```

The program looks for shaders in `shaders/archetype_convolution/`, then in the current directory, then in `../../shaders/`. It looks for media in `./images` and `../../images`.

## Requirements (Fedora)

Build tools and the FFmpeg development files (the `ffmpeg-next` crate links against them):

```bash
sudo dnf install gcc clang clang-devel pkgconf-pkg-config
```

Then install FFmpeg development files. Pick one:

**RPM Fusion (recommended, plays ordinary `.mp4` / H.264 files):**

```bash
sudo dnf install https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm
sudo dnf swap ffmpeg-free ffmpeg --allowerasing
sudo dnf install ffmpeg-devel
```

**Fedora packages only (fewer codecs, some `.mp4` files may not play):**

```bash
sudo dnf install ffmpeg-free-devel
```

Do not install both `-devel` packages, they conflict.

Check which FFmpeg version you have, because the Rust crate's major version must match it:

```bash
pkg-config --modversion libavcodec
```

`61.x` means FFmpeg 7 (use `ffmpeg-next = "7"`). `62.x` means FFmpeg 8 (use `ffmpeg-next = "8"`).

You also need Rust (install with [rustup](https://rustup.rs)) and a GPU/driver that supports OpenGL ES 3.0 (any recent Mesa or NVIDIA driver does).

## Cargo.toml

`vc.rs` must be built as a library called `vc`, and `ffmpeg-next` must match your FFmpeg version:

```toml
[package]
name = "archetypes"
version = "0.1.0"
edition = "2021"

[lib]
name = "vc"
path = "src/vc.rs"

[dependencies]
glfw = "0.60"        # keep the versions your project already uses
glow = "0.13"
image = "0.25"
ffmpeg-next = { version = "8", default-features = false, features = ["codec", "format", "software-scaling"] }
```

Binaries in `src/bin/` are found automatically, so no `[[bin]]` entries are needed.

## Run

From the `archetypes` folder:

```bash
cargo run --bin archetype_image_convolution
```

The first build takes a while because the FFmpeg bindings are generated. The program prints every file it found and opens a window on the first one.

Optional argument:

```bash
# start on a specific file (by name, name without extension, or path)
cargo run --bin archetype_image_convolution -- cat.png

# use a different folder of images and videos
cargo run --bin archetype_image_convolution -- ~/Pictures/test
```

Supported files:

- Images: `png`, `jpg`, `jpeg`, `bmp`, `gif`, `tiff`
- Videos: `mp4`, `mov`, `mkv`, `webm`, `avi`, `m4v`

Videos loop and have no audio.

## Controls

| Key | Action |
|---|---|
| Up / Down | Next / previous image or video |
| Left / Right | Previous / next kernel |
| 1 to 9 | Jump straight to kernel 1 to 9 |
| Space | Toggle the original (unfiltered) picture |
| P | Pause / resume video |
| `[` / `]` | Lower / raise bias (hold the key) |
| R | Reset bias |
| Esc | Quit |

The window title shows the current file, kernel and bias.

## Kernels

| # | Kernel | What it does |
|---|---|---|
| 0 | Ridge (Sobel X) | Vertical edges on mid-gray |
| 1 | Emboss | 3D relief look |
| 2 | Sharpen | Slightly crisper detail |
| 3 | Box blur | Uniform 3x3 blur |
| 4 | Identity | Same as the original |
| 5 | Gaussian blur | Softer blur than the box |
| 6 | Sobel Y | Horizontal edges on mid-gray |
| 7 | Laplacian | Edges in all directions on gray |
| 8 | Outline | White edges on black |
| 9 | Sharpen strong | Harsher sharpening |
| 10 | Motion blur (diagonal) | Streak along one diagonal |
| 11 | Sobel magnitude | X and Y edges combined |

Kernels that are centred on 0 (Ridge, Emboss, Sobel Y, Laplacian) get a base bias of 0.5 so flat areas appear gray instead of black.

### Adding your own kernel

1. In `archetype_convolution.frag`, add a `const float kName[9]` array and an `if (k == N) return kName[i];` line in `weightFor`.
2. In `archetype_image_convolution.rs`, add a name to `KERNEL_NAMES` and a value to `KERNEL_BIAS`.
3. Update the `12` in the type of both arrays to the new count.

## Troubleshooting

**`no matching package named vc found`**
`vc` is part of this project, not a separate crate. Remove any `vc = { path = ... }` line from `Cargo.toml` and make sure the `[lib]` section above is present.

**`'/usr/include/libavcodec/avfft.h' file not found`**
The `ffmpeg-next` version does not match your FFmpeg. FFmpeg 8 removed that header, so use `ffmpeg-next = "8"` when `pkg-config --modversion libavcodec` reports `62.x`, then run `cargo clean -p ffmpeg-sys-next` and build again.

**A video does not open or the program skips it**
The message is printed in the terminal. With the Fedora-only `ffmpeg-free` packages, H.264 and H.265 files often cannot be decoded. Switch to the RPM Fusion packages above, or convert the file to WebM (VP9).

**`no images or videos found`**
Put files in `./images` or `../../images`, or pass a folder as the argument. The program searches relative to the directory you run `cargo run` from.

**Could not find shader file**
Check that both `.vert` and `.frag` files are in `shaders/archetype_convolution/` and are named `archetype_convolution.vert` and `archetype_convolution.frag`.

**The picture looks black or washed out**
Use the `[` and `]` keys to adjust the bias, or press R to reset it. Edge kernels produce values near zero, so their appearance depends heavily on the bias.

**Blank or corrupted window**
Make sure the fragment shader is the updated version with `uKernel` declared as `int` and 12 kernels. An older shader with `uniform float uKernel` will not work with this program.