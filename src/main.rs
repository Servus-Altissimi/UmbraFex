//   ▄▄▄  ▄▄                             ▄▄▄▄▄▄▄          
//  █▀██  ██           █▄               █▀██▀▀▀           
//    ██  ██  ▄        ██    ▄            ██              
//    ██  ██  ███▄███▄ ████▄ ████▄▄▀▀█▄   ███▀▄█▀█▄▀██ ██▀
//    ██  ██  ██ ██ ██ ██ ██ ██   ▄█▀██ ▄ ██  ██▄█▀  ███  
//    ▀█████▄▄██ ██ ▀█▄████▀▄█▀  ▄▀█▄██ ▀██▀ ▄▀█▄▄▄▄██ ██▄
                                                                              
// write shaders, crash tab, look cute doing it <3
// Inspired by ShaderToy

// Copyright 2026 Servus Altissimi (Pseudonym)

// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#![allow(non_snake_case)]

use std::borrow::Cow;
use std::cell::RefCell;

use bytemuck::{Pod, Zeroable};
use dioxus::document::eval;
use dioxus::prelude::*;
use futures_channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen::JsCast;
use wgpu::util::DeviceExt;

const STYLE: Asset = asset!("assets/style.scss");

const DRAG_V_JS: &str = "(function(){
    const p = document.querySelector('.pane-errors');
    let y0 = event.clientY, h0 = p.getBoundingClientRect().height;
    const mm = e => p.style.height = Math.max(24, h0 - (e.clientY - y0)) + 'px';
    const mu = () => {
        removeEventListener('mousemove', mm);
        removeEventListener('mouseup', mu);
    };
    addEventListener('mousemove', mm);
    addEventListener('mouseup', mu);
})();";

const DRAG_H_JS: &str = "(function(){
    const r = document.querySelector('.panel-right');
    let x0 = event.clientX, w0 = r.getBoundingClientRect().width;
    const mm = e => {
        r.style.width = Math.max(200, w0 - (e.clientX - x0)) + 'px';
        r.style.flex = 'none';
    };
    const mu = () => {
        removeEventListener('mousemove', mm);
        removeEventListener('mouseup', mu);
    };
    addEventListener('mousemove', mm);
    addEventListener('mouseup', mu);
})();";

const SYNC_SCROLL_JS: &str = "
    const ta = document.querySelector('.code');
    const g  = document.querySelector('.gutter');
    g.scrollTop = ta.scrollTop;
";

const DEFAULT_SHADER: &str = r#"// Hecko (˶ᵔᗜᵔ˶)ﾉﾞ 
// I like to call this one, performant rings.
// Feel free to mess around with it, or write your own shaders!


struct Uniforms {
    resolution: vec2<f32>,
    time:       f32,
    _pad:       f32,
}
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@location(0) p: vec2<f32>) -> VOut {
    return VOut(vec4<f32>(p, 0.0, 1.0), p * 0.5 + 0.5);
}

const PASTEL_PINK  = vec3<f32>(0.95, 0.73, 0.82);
const PASTEL_MINT  = vec3<f32>(0.69, 0.87, 0.81);
const PASTEL_BLUE  = vec3<f32>(0.70, 0.82, 0.93);
const PASTEL_PEACH = vec3<f32>(0.97, 0.83, 0.72);
const PASTEL_CREAM = vec3<f32>(0.97, 0.95, 0.91);

fn sdTorus(p: vec3<f32>, r_major: f32, r_minor: f32) -> f32 {
    let q = vec2<f32>(length(p.xz) - r_major, p.y);
    return length(q) - r_minor;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let aspect = u.resolution.x / u.resolution.y;
    let fov    = 1.2;
    let ro     = vec3<f32>(0.0, 0.0, -3.5);
    let rd     = normalize(vec3<f32>(
        (in.uv.x - 0.5) * aspect * fov,
        (in.uv.y - 0.5) * fov,
        1.0
    ));

    const MAJOR_RADII  = array<f32, 4>(0.65, 0.80, 0.95, 1.10);
    const MINOR_RADII  = array<f32, 4>(0.08, 0.07, 0.09, 0.08);
    const ROT_OFFSETS  = array<f32, 4>(0.0,  1.57, 3.14, 4.71);
    let   colours      = array<vec3<f32>, 4>(PASTEL_PINK, PASTEL_MINT, PASTEL_BLUE, PASTEL_PEACH);

    let t_base = u.time * 0.35;
    let t_alt  = u.time * 0.25;

    var ray_t   = 0.0;
    var hit_idx = -1;

    for (var i = 0; i < 32; i++) {
        let pos   = ro + rd * ray_t;
        var min_d = 1e6;
        var min_i = 0;

        for (var ring = 0; ring < 4; ring++) {
            let a1 = t_base + ROT_OFFSETS[ring];
            let a2 = t_alt  + ROT_OFFSETS[ring] * 0.7;
            let c1 = cos(a1); let s1 = sin(a1);
            let c2 = cos(a2); let s2 = sin(a2);

            var lp = vec3<f32>(
                pos.x * c1 + pos.z * s1,
                pos.y,
               -pos.x * s1 + pos.z * c1
            );
            lp = vec3<f32>(
                lp.x,
                lp.y * c2 - lp.z * s2,
                lp.y * s2 + lp.z * c2
            );

            let d = sdTorus(lp, MAJOR_RADII[ring], MINOR_RADII[ring]);
            if d < min_d { min_d = d; min_i = ring; }
        }

        if min_d < 0.01 { hit_idx = min_i; break; }
        if ray_t > 6.0  { break; }
        ray_t += max(min_d * 0.7, 0.02);
    }

    if hit_idx >= 0 {
        var col  = colours[hit_idx];
        let glow = exp(-ray_t * ray_t * 0.04) * 0.12;
        col += colours[hit_idx] * glow;
        return vec4<f32>(col, 1.0);
    }
    return vec4<f32>(PASTEL_CREAM, 1.0);
}"#;

const CANVAS_ID: &str = "canvas";

// TX_SLOT / RX_SLOT form a one-shot channel used to hand the shader
// source from the Dioxus component tree (which owns the editor state) down
// into the render coroutine that lives outside the component lifecycle.
// Using thread_local! + RefCell here because wasm is single-threaded, so
// there is no need for Arc<Mutex<_>>; take() is used to move the receiver
// into the coroutine exactly once, leaving None behind to prevent accidental
// double-takes.

// ERR_TX / ERR_RX is the reverse path: the GPU/render coroutine
// sends compilation errors (or an empty string on success) back up so the
// component can display them in the error pane without any shared mutable
// state visible to the component layer.
thread_local! {
    static TX_SLOT: RefCell<Option<UnboundedSender<String>>>   = RefCell::new(None);
    static RX_SLOT: RefCell<Option<UnboundedReceiver<String>>> = RefCell::new(None);
    static ERR_TX:  RefCell<Option<UnboundedSender<String>>>   = RefCell::new(None);
    static ERR_RX:  RefCell<Option<UnboundedReceiver<String>>> = RefCell::new(None);
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms { resolution: [f32; 2], time: f32, _pad: f32 }

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Vertex { pos: [f32; 2] }

const QUAD: &[Vertex] = &[
    Vertex { pos: [-1.0, -1.0] }, Vertex { pos: [ 1.0, -1.0] },
    Vertex { pos: [-1.0,  1.0] }, Vertex { pos: [ 1.0, -1.0] },
    Vertex { pos: [ 1.0,  1.0] }, Vertex { pos: [-1.0,  1.0] },
];

struct Gpu {
    surface:    wgpu::Surface<'static>,
    device:     wgpu::Device,
    queue:      wgpu::Queue,
    pipeline:   wgpu::RenderPipeline,
    vtx_buf:    wgpu::Buffer,
    uni_buf:    wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bgl:        wgpu::BindGroupLayout,
    config:     wgpu::SurfaceConfiguration,
    format:     wgpu::TextureFormat,
    start:      f64,
    canvas:     web_sys::HtmlCanvasElement,
}

impl Gpu {
    async fn new(canvas: web_sys::HtmlCanvasElement, shader_src: &str) -> Result<Self, String> {
        let w = canvas.client_width()  as u32;
        let h = canvas.client_height() as u32;
        canvas.set_width(w.max(1));
        canvas.set_height(h.max(1));

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| format!("Surface: {e}"))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface:     Some(&surface),
                power_preference:       wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| format!("Adapter: {e}"))?;

        let (device, queue): (wgpu::Device, wgpu::Queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label:                 None,
                required_features:     wgpu::Features::empty(),
                required_limits:       wgpu::Limits::downlevel_webgl2_defaults(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints:          Default::default(),
                trace:                 wgpu::Trace::Off,
            })
            .await
            .map_err(|e| format!("Device: {e}"))?;

        let caps   = surface.get_capabilities(&adapter);
        let format = caps.formats.first().copied().ok_or("No surface formats")?;
        let config  = wgpu::SurfaceConfiguration {
            usage:                         wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width:                         w.max(1),
            height:                        h.max(1),
            present_mode:                  wgpu::PresentMode::AutoVsync,
            alpha_mode:                    caps.alpha_modes[0],
            view_formats:                  vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let vtx_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vtx"), contents: bytemuck::cast_slice(QUAD),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let uni_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uni"), size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0, visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false, min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg"), layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0, resource: uni_buf.as_entire_binding(),
            }],
        });
        let pipeline = build_pipeline(&device, shader_src, format, &bgl).await?;
        let start    = web_sys::window().unwrap().performance().unwrap().now();

        Ok(Gpu { surface, device, queue, pipeline, vtx_buf, uni_buf,
                  bind_group, bgl, config, format, start, canvas })
    }

    fn maybe_resize(&mut self) {
        let cw = self.canvas.client_width()  as u32;
        let ch = self.canvas.client_height() as u32;
        if cw < 1 || ch < 1 { return; }
        if cw == self.config.width && ch == self.config.height { return; }
        self.canvas.set_width(cw);
        self.canvas.set_height(ch);
        self.config.width  = cw;
        self.config.height = ch;
        self.surface.configure(&self.device, &self.config);
    }

    async fn rebuild(&mut self, src: &str) -> Result<(), String> {
        self.pipeline = build_pipeline(&self.device, src, self.format, &self.bgl).await?;
        Ok(())
    }

    fn render(&mut self) {
        self.maybe_resize();

        let t = ((web_sys::window().unwrap().performance().unwrap().now() - self.start) / 1000.0) as f32;
        let u = Uniforms {
            resolution: [self.config.width as f32, self.config.height as f32],
            time: t, _pad: 0.0,
        };
        self.queue.write_buffer(&self.uni_buf, 0, bytemuck::cast_slice(&[u]));

        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(_) => return,
        };
        let view = frame.texture.create_view(&Default::default());
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view, resolve_target: None,
                    ops: wgpu::Operations {
                        load:  wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                multiview_mask: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.vtx_buf.slice(..));
            pass.draw(0..6, 0..1);
        }
        self.queue.submit(Some(enc.finish()));
        frame.present();
    }
}

// Build pipeline and capture shader errors
async fn build_pipeline(
    device: &wgpu::Device,
    src:    &str,
    format: wgpu::TextureFormat,
    bgl:    &wgpu::BindGroupLayout,
) -> Result<wgpu::RenderPipeline, String> {
    // Push a validation error scope so shader errors are captured
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("s"),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(src.to_string())),
    });

    // Check compilation info for errors with line numbers
    let info = shader.get_compilation_info().await;
    let errors: Vec<String> = info.messages.iter()
        .filter(|m| m.message_type == wgpu::CompilationMessageType::Error)
        .map(|m| format!("line {}: {}", m.location.map_or(0, |l| l.line_number), m.message))
        .collect();

    // Pop error scope via the handle returned by push_error_scope
    let _ = scope.pop().await;    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("pl"), bind_group_layouts: &[bgl], immediate_size: 0,
    });
    Ok(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("rp"), layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader, entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2,
                }],
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader, entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None, ..Default::default()
        },
        depth_stencil: None, multisample: Default::default(),
        multiview_mask: None, cache: None,
    }))
}

fn main() {
    let (tx,  rx)  = mpsc::unbounded::<String>();
    let (etx, erx) = mpsc::unbounded::<String>();
    TX_SLOT.with(|s| *s.borrow_mut() = Some(tx));
    RX_SLOT.with(|s| *s.borrow_mut() = Some(rx));
    ERR_TX.with(|s|  *s.borrow_mut() = Some(etx));
    ERR_RX.with(|s|  *s.borrow_mut() = Some(erx));
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut src   = use_signal(|| DEFAULT_SHADER.to_string());
    let mut error = use_signal(|| String::new());

    let tx: UnboundedSender<String> = use_hook(||
        TX_SLOT.with(|s| s.borrow().as_ref().unwrap().clone())
    );

    // Render coroutine
    use_coroutine(|_: UnboundedReceiver<()>| async move {
        let mut rx = RX_SLOT.with(|s| s.borrow_mut().take()).expect("RX_SLOT");

        let canvas = loop {
            TimeoutFuture::new(50).await;
            let doc = web_sys::window().unwrap().document().unwrap();
            if let Some(el) = doc.get_element_by_id(CANVAS_ID) {
                if let Ok(c) = el.dyn_into::<web_sys::HtmlCanvasElement>() {
                    if c.client_width() > 0 { break c; }
                }
            }
        };

        let mut gpu = match Gpu::new(canvas, DEFAULT_SHADER).await {
            Ok(g)  => g,
            Err(e) => {
                ERR_TX.with(|s| { s.borrow().as_ref().map(|t| { let _ = t.unbounded_send(e.clone()); }); });
                return;
            }
        };

        loop { 
            while let Ok(src) = rx.try_recv() {
                let res = gpu.rebuild(&src).await;
                ERR_TX.with(|s| { s.borrow().as_ref().map(|t| {
                    let _ = t.unbounded_send(res.err().unwrap_or_default());
                }); });
            }
            gpu.render();
            TimeoutFuture::new(16).await;
        }
    });

    // Error poll coroutine
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        let mut erx = ERR_RX.with(|s| s.borrow_mut().take()).expect("ERR_RX");
        loop {
            if let Ok(msg) = erx.try_recv() { error.set(msg); }
            TimeoutFuture::new(100).await;
        }
    });

    let on_run = move |_| {
        error.set(String::new());
        let _ = tx.unbounded_send(src.read().clone());
    };

    let on_fullscreen = move |_| {
        let _ = eval(r#"
            const w = document.getElementById('canvas-wrap');
            if (!document.fullscreenElement) w.requestFullscreen();
            else document.exitFullscreen();
        "#);
    };
 
    let line_count = use_memo(move || src.read().lines().count().max(1));

    rsx! {
        document::Stylesheet { href: STYLE }
        div { class: "root",
            // Left: canvas + errors
            div { class: "panel-left",
                div { id: "canvas-wrap", class: "pane-canvas",
                    canvas { id: CANVAS_ID }
                }
                div {
                    class: "drag-v",
                    onmousedown: move |_| { let _ = eval(DRAG_V_JS); }
                }
                div { class: "pane-errors",
                    if error.read().is_empty() {
                        div { class: "no-err", "no errors" }
                    } else {
                        div { class: "err-inner", "{error}" }
                    }
                }
            }

            div {
                class: "drag-h",
                onmousedown: move |_| { let _ = eval(DRAG_H_JS); }
            }

            // Right: editor
            div { class: "panel-right",
                div { class: "bar",
                    button { onclick: on_run, "Run" }
                    button { onclick: on_fullscreen, "Fullscreen" }
                }
                div { class: "editor-wrap",
                    div { class: "gutter",
                        for n in 1..=*line_count.read() {
                            div { class: "gutter-line", "{n}" }
                        }
                    }
                    textarea {
                        class: "code",
                        spellcheck: false,
                        value: "{src}",
                        oninput: move |e| src.set(e.value()),
                        onscroll: move |_| { let _ = eval(SYNC_SCROLL_JS); },
                    }
                }
            }
        }
    }
}
