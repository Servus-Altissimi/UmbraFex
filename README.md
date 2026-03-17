# UmbraFex

```
  ▄▄▄  ▄▄                             ▄▄▄▄▄▄▄          
 █▀██  ██           █▄               █▀██▀▀▀           
   ██  ██  ▄        ██    ▄            ██              
   ██  ██  ███▄███▄ ████▄ ████▄▄▀▀█▄   ███▀▄█▀█▄▀██ ██▀
   ██  ██  ██ ██ ██ ██ ██ ██   ▄█▀██ ▄ ██  ██▄█▀  ███  
   ▀█████▄▄██ ██ ▀█▄████▀▄█▀  ▄▀█▄██ ▀██▀ ▄▀█▄▄▄▄██ ██▄
```

**A WebGPU-powered WGSL shader editor that runs entirely in the browser.** 

  [![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
  [![Rust](https://img.shields.io/badge/rust-1.89+-orange.svg)](https://www.rust-lang.org/)
  
<img width="1870" height="916" alt="image" src="https://github.com/user-attachments/assets/03129092-2291-4d32-bb2d-ccb70d1f90f4" />

--- 

Inspired by [ShaderToy](https://www.shadertoy.com/). Built with [Dioxus](https://dioxuslabs.com/) and [WGPU](https://wgpu.rs/), runs entirely in the browser via WebGPU.

## Features
- Live WGSL shader editor
- Real-time compilation errors
- Uniforms for resolution and time passed automatically
- Resizable editor and error pane via drag handles
- Fullscreen canvas mode
- Ships with a cute default pastel torus raymarcher to get you started 

## Requirements
- A browser that supports WebGPU ([You may need to enable it](https://enablegpu.com/)
- Rust & Cargo
- [Dioxus](https://dioxuslabs.com/)

## Use
```bash
dx serve --platform web
```

## Writing Shaders

Shaders are written in [WGSL](https://gpuweb.github.io/gpuweb/wgsl/). The following uniforms are available:

```wgsl
struct Uniforms {
    resolution: vec2<f32>,  // canvas size in pixels
    time:       f32,        // seconds since launch
    _pad:       f32,
}
@group(0) @binding(0) var<uniform> u: Uniforms;
```

Your shader needs a vertex entry point `vs_main` and a fragment entry point `fs_main`. The default shader is a good starting point.

Hit `Run` to recompile. Errors show up in the pane below the canvas.
