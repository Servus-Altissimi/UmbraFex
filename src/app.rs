use dioxus::document::eval;
use dioxus::prelude::*;
use futures_channel::mpsc::UnboundedReceiver;
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen::JsCast;

use crate::gpu::Gpu;
use crate::highlight::{highlight_wgsl, parse_err_lines};
use crate::js;
use crate::{ERR_RX, ERR_TX, RX_SLOT, TX_SLOT};
use crate::components::editor::Editor;
use crate::components::error_pane::ErrorPane;
use crate::components::toolbar::Toolbar;

const STYLE: Asset = asset!("assets/style.scss");

const CANVAS_ID: &str = "canvas";

pub const DEFAULT_SHADER: &str = r#"// Hecko (˶ᵔᗜᵔ˶)ﾉﾞ 
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

#[component]
pub fn App() -> Element {
    let mut src   = use_signal(|| DEFAULT_SHADER.to_string());
    let mut error = use_signal(|| String::new());

    let tx = use_hook(||
        TX_SLOT.with(|s| s.borrow().as_ref().unwrap().clone())
    );

    // Recomputed reactively whenever src changes
    let highlighted = use_memo(move || highlight_wgsl(&src.read(), &parse_err_lines(&error.read())));

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
                    onmousedown: move |_| { let _ = eval(js::DRAG_V); }
                }
                ErrorPane { error: error.read().clone() }
            }

            div {
                class: "drag-h",
                onmousedown: move |_| { let _ = eval(js::DRAG_H); }
            }

            // Right: editor
            div { class: "panel-right",
                Toolbar { on_run, on_fullscreen }
                Editor {
                    src: src.read().clone(),
                    highlighted: highlighted.read().clone(),
                    on_input: move |v| src.set(v),
                }
            }
        }
    }
}
