use dioxus::document::eval;
use dioxus::prelude::*;
use futures_channel::mpsc::UnboundedReceiver;
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen::JsCast;

use crate::gpu::{Gpu, PerfStats, TimelineCmd};
use crate::highlight::{highlight_wgsl, parse_err_lines};
use crate::js;

use crate::{ERR_RX, ERR_TX, PERF_RX, PERF_TX, RX_SLOT, TX_SLOT, TIMELINE_RX, TIMELINE_TX};

const STYLE: Asset = asset!("assets/style.scss");
const NO_WEBGPU: Asset = asset!("assets/nowebgpu.svg");
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

// Pane identifier, each value is one dockable panel
#[derive(Clone, PartialEq, Debug, Copy)]
pub enum PaneId {
    Canvas,
    Editor,
    Errors,
    Perf,

    Timeline,
}

impl PaneId {
    fn label(self) -> &'static str {
        match self {
            Self::Canvas   => "Canvas",
            Self::Editor   => "Editor",
            Self::Errors   => "Tools",
            Self::Perf     => "Performance",
            Self::Timeline => "Timeline",
        }
    }
}

// Each zone holds an ordered list of panes and tracks which tab is active.
#[derive(Clone)]
struct DockState {
    zones:  [Vec<PaneId>; 3],
    active: [usize; 3],
}

impl DockState {
    fn new() -> Self {
        Self {
            zones:  [
                vec![PaneId::Canvas], 
                vec![PaneId::Errors, PaneId::Perf], 
                vec![PaneId::Editor]
            ],
            active: [0, 0, 0],
        }
    }
    
    fn active_pane(&self, z: usize) -> Option<PaneId> {
        self.zones[z].get(self.active[z]).copied()
    }
    
    fn move_pane(&mut self, pane: PaneId, from: usize, to: usize) {
        if from == to { return; }
        if let Some(pos) = self.zones[from].iter().position(|&p| p == pane) {
            self.zones[from].remove(pos);
            if self.active[from] >= self.zones[from].len() && !self.zones[from].is_empty() {
                self.active[from] = self.zones[from].len() - 1;
            }
        }
        if !self.zones[to].contains(&pane) {
            self.zones[to].push(pane);
            self.active[to] = self.zones[to].len() - 1;
        }
    }

    // Remove a pane from whichever zone contains it, adjusting active indices.
    fn remove_pane(&mut self, pane: PaneId) {
        for z in 0..3 {
            if let Some(pos) = self.zones[z].iter().position(|&p| p == pane) {
                self.zones[z].remove(pos);
                if !self.zones[z].is_empty() && self.active[z] >= self.zones[z].len() {
                    self.active[z] = self.zones[z].len() - 1;
                }
            }
        }
    }
}

#[component]
pub fn App() -> Element {
    let mut webgpu_ok = use_signal(|| true);
    let mut src       = use_signal(|| DEFAULT_SHADER.to_string());
    let mut error    = use_signal(|| String::new());
    let mut perf     = use_signal(PerfStats::default);
    let mut dock     = use_signal(DockState::new);
    let mut dragging: Signal<Option<(PaneId, usize)>> = use_signal(|| None);
    let mut drag_ov:  Signal<Option<usize>>           = use_signal(|| None);
 
    let mut tl_enabled:     Signal<bool> = use_signal(|| false);
    let mut tl_duration:    Signal<f32>  = use_signal(|| 10.0f32);
    let mut tl_playing:     Signal<bool> = use_signal(|| false);

    // Display position is updated immediately on scrub (for instant feedback)
    // and also fed back by the render coroutine via PerfStats for smooth playback display.
    let mut tl_display_pos: Signal<f32>  = use_signal(|| 0.0f32);

    let tx          = use_hook(|| TX_SLOT.with(|s| s.borrow().as_ref().unwrap().clone()));
    let timeline_tx = use_hook(|| TIMELINE_TX.with(|s| s.borrow().as_ref().unwrap().clone()));

    use_effect(move || {
        spawn(async move {
            if let Ok(val) = eval("navigator.gpu !== undefined").await {
                if val.as_bool() == Some(false) {
                    webgpu_ok.set(false);
                }
            }
        });
    });

    // Recomputed reactively whenever src changes
    let highlighted = use_memo(move || highlight_wgsl(&src.read(), &parse_err_lines(&error.read())));

    use_effect(move || {
        let current_src = src.read().clone();
        let has_changes = current_src != DEFAULT_SHADER;
        
        if has_changes {
            let _ = eval(js::ENABLE_BEFOREUNLOAD);
        } else {
            let _ = eval(js::DISABLE_BEFOREUNLOAD);
        }
    });

    let tl_sync_tx = timeline_tx.clone();
    use_effect(move || {
        let enabled  = *tl_enabled.read();
        let duration = *tl_duration.read();
        let playing  = *tl_playing.read();
        let _ = tl_sync_tx.unbounded_send(TimelineCmd {
            enabled,
            duration,
            playing,
            seek_to: None, // not a seek, just a state sync
        });
    });

    // Render coroutine
    use_coroutine(|_: UnboundedReceiver<()>| async move {
        let mut rx = RX_SLOT.with(|s| s.borrow_mut().take()).expect("RX_SLOT"); 
        let mut timeline_rx = TIMELINE_RX.with(|s| s.borrow_mut().take()).expect("TIMELINE_RX");

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

        // Rolling 60 sample window for FPS, push stats every 5 frames
        let perf_obj = web_sys::window().unwrap().performance().unwrap();
        let mut ftimes: std::collections::VecDeque<f64> = std::collections::VecDeque::with_capacity(60);
        let mut last = perf_obj.now();
        let mut tick = 0u32;
 
        let mut tl_enabled  = false;
        let mut tl_duration = 10.0f32;
        let mut tl_pos      = 0.0f32;
        let mut tl_playing  = false;

        loop {
            while let Ok(cmd) = timeline_rx.try_recv() {
                tl_enabled  = cmd.enabled;
                tl_duration = cmd.duration.max(0.1);
                tl_playing  = cmd.playing;
                if let Some(pos) = cmd.seek_to {
                    tl_pos = pos.clamp(0.0, tl_duration);
                }
            }

            while let Ok(s) = rx.try_recv() {
                let res = gpu.rebuild(&s).await;
                ERR_TX.with(|s| { s.borrow().as_ref().map(|t| {
                    let _ = t.unbounded_send(res.err().unwrap_or_default());
                }); });
            }

            // Compute frame delta for both FPS tracking and timeline advancement
            let now      = perf_obj.now();
            let delta_ms = now - last;
            last = now;

            // Advance the timeline position by the real elapsed frame time,
            // then wrap around so the animation loops seamlessly.
            if tl_enabled && tl_playing && tl_duration > 0.0 {
                let delta_s = (delta_ms / 1000.0) as f32;
                tl_pos = (tl_pos + delta_s) % tl_duration;
            }
 
            gpu.render(if tl_enabled { Some(tl_pos) } else { None });

            ftimes.push_back(delta_ms);
            if ftimes.len() > 60 { ftimes.pop_front(); }
            tick += 1;

            if tick % 5 == 0 {
                let avg = ftimes.iter().sum::<f64>() / ftimes.len() as f64;
                PERF_TX.with(|s| { s.borrow().as_ref().map(|t| {
                    let _ = t.unbounded_send(PerfStats {
                        fps:          (1000.0 / avg) as f32,
                        frame_ms:     avg as f32,
                        w:            gpu.config.width,
                        h:            gpu.config.height,
                        gpu_name:     gpu.gpu_name.clone(),
                        backend:      gpu.backend.clone(),
                        timeline_pos: tl_pos,
                    });
                }); });
            }

            TimeoutFuture::new(16).await;
        }
    });

    // Error poll coroutine
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        let mut erx = ERR_RX.with(|s| s.borrow_mut().take()).expect("ERR_RX");
        loop {
            if let Ok(msg) = erx.try_recv() {
                // Fatal GPU initialisation errors (adapter/surface/device failures)
                // mean WebGPU is not actually usable: flip the flag so the
                // no-WebGPU pane renders instead of a silent blank canvas.
                if msg.starts_with("Adapter:")
                    || msg.starts_with("Surface:")
                    || msg.starts_with("Device:")
                {
                    webgpu_ok.set(false);
                }
                error.set(msg);
            }
            TimeoutFuture::new(100).await;
        }
    });

    // Performance stats poll coroutine
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        let mut prx = PERF_RX.with(|s| s.borrow_mut().take()).expect("PERF_RX");
        loop {
            if let Ok(s) = prx.try_recv() {
                tl_display_pos.set(s.timeline_pos);
                perf.set(s);
            } 
            TimeoutFuture::new(50).await; // Magic number, smooth
        }
    });

    // Start a RAF loop, runs once, that continuously
    // mirrors #canvas-slot's bounding rect onto #canvas-fixed. This keeps the
    // real canvas overlay perfectly in sync without ever touching the DOM node
    // that wgpu holds a surface reference to.
    use_effect(move || {
        let _ = eval(js::CANVAS_SYNC); 
    });

    // Builds tab Elements for one zone outside rsx
    let make_tabs = move |idx: usize| -> Vec<Element> {
        let d        = dock.read();
        let panes    = d.zones[idx].clone();
        let active_i = d.active[idx];
        drop(d);
        panes.into_iter().enumerate().map(|(i, pane)| {
            let cls = if i == active_i { "zone-tab zone-tab-active" } else { "zone-tab" };
            rsx! {
                div {
                    key: "{pane:?}",
                    class: cls,
                    draggable: true,
                    ondragstart: move |_| { dragging.set(Some((pane, idx))); },
                    // ondragend active even when dropped outside any zone
                    ondragend:   move |_| { dragging.set(None); drag_ov.set(None); },
                    onclick:     move |_| { dock.write().active[idx] = i; },
                    "{pane.label()}"
                }
            }
        }).collect()
    };

    // Builds one zone element (tab strip + active pane) by index.
    let zone = move |idx: usize, extra: &'static str| -> Element {
        let d        = dock.read();
        let panes    = d.zones[idx].clone();
        let active_p = d.active_pane(idx);
        drop(d);
        let is_over = drag_ov.read().map_or(false, |z| z == idx);
        let cls     = format!("zone {extra}{}", if is_over { " zone-drop" } else { "" });
        let tabs    = make_tabs(idx);
        
        let contents: Vec<Element> = panes.into_iter().map(|pane| {
            let visible = Some(pane) == active_p;
            let style   = if visible { "display:flex;flex:1 1 0;min-height:0;flex-direction:column;" } else { "display:none;" }; 
            let tx_clone = tx.clone();
            rsx! {
                div { key: "{pane:?}", style,
                    match pane {
                        PaneId::Canvas => rsx! {
                            if *webgpu_ok.read() {
                                div { id: "canvas-slot", class: "pane-canvas" }
                            } else {
                                div { class: "pane-canvas pane-no-webgpu",
                                    img { src: NO_WEBGPU, alt: "No WebGPU" }
                                    p { "WebGPU isn't available in your browser." }
                                    p {
                                        "Enable it: "
                                        a {
                                            href: "https://enablegpu.com/",
                                            target: "_blank",
                                            "https://enablegpu.com/"
                                        }
                                    }
                                }      
                            }

                        },
                        PaneId::Editor => rsx! {
                            crate::components::editor::Editor {
                                src: src.read().clone(),
                                highlighted: highlighted.read().clone(),
                                on_input: move |v| src.set(v),
                            }
                        },
                        PaneId::Errors => rsx! {
                            crate::components::error_pane::ErrorPane {
                                error:              error.read().clone(),
                                timeline_enabled:   *tl_enabled.read(),
                                on_run: move |_| {
                                    error.set(String::new());
                                    let _ = tx_clone.unbounded_send(src.read().clone());
                                },
                                on_fullscreen: move |_| {
                                    let _ = eval(js::FS_TOGGLE);
                                }, 
                                on_timeline_toggle: move |_| { // Toggle timeline
                                    let new_enabled = !*tl_enabled.read();
                                    tl_enabled.set(new_enabled);
                                    if new_enabled {
                                        // Add Timeline tab to zone 1 if absent, then focus it
                                        let mut d = dock.write();
                                        if !d.zones[1].contains(&PaneId::Timeline) {
                                            d.zones[1].push(PaneId::Timeline);
                                        }
                                        let idx = d.zones[1]
                                            .iter()
                                            .position(|&p| p == PaneId::Timeline)
                                            .unwrap_or(0);
                                        d.active[1] = idx;
                                    } else { 
                                        tl_playing.set(false);
                                        let mut d = dock.write();
                                        d.remove_pane(PaneId::Timeline);
                                    }
                                },
                            }
                        },
                        PaneId::Perf => rsx! {
                            crate::components::perf_pane::PerfPane { stats: perf.read().clone() }
                        },

                        PaneId::Timeline => { 
                            let tl_seek_tx = timeline_tx.clone();
                            rsx! {
                                crate::components::timeline_pane::TimelinePane {
                                    duration:  *tl_duration.read(),
                                    position:  *tl_display_pos.read(),
                                    playing:   *tl_playing.read(),
                                    on_duration_change: move |v: f32| tl_duration.set(v), 
                                    on_seek: move |pos: f32| {
                                        tl_display_pos.set(pos);
                                        let _ = tl_seek_tx.unbounded_send(TimelineCmd {
                                            enabled:  *tl_enabled.read(),
                                            duration: *tl_duration.read(),
                                            playing:  *tl_playing.read(),
                                            seek_to:  Some(pos),
                                        });
                                    },
                                    on_play_pause: move |playing: bool| tl_playing.set(playing),
                                }
                            }
                        },
                    }
                }
            }
        }).collect();

        rsx! {
            div {
                class: cls, 
                ondragover: move |e| { e.prevent_default(); drag_ov.set(Some(idx)); },
                ondrop: move |e| {
                    e.prevent_default();
                    if let Some((pane, from)) = *dragging.read() {
                        dock.write().move_pane(pane, from, idx);
                    }
                    dragging.set(None);
                    drag_ov.set(None);
                },
                div { class: "zone-tabs", { tabs.into_iter() } }
                div { class: "zone-content", { contents.into_iter() } }
            }
        }
    };

    rsx! {
        document::Stylesheet { href: STYLE }
        // Canvas lives here permanently at the root, outside Dioxus's zonetree.
        // The RAF loop in use_effect positions this over #canvas-slot so it appears
        // in the right place, but is never unmounted or reparented by Dioxus.
        div { id: "canvas-fixed", canvas { id: CANVAS_ID } }
        div { class: "root",
            // Left: two zones stacked vertically
            div { class: "panel-left",
                { zone(0, "zone-grow") }
                div { class: "drag-v", onmousedown: move |_| { let _ = eval(js::DRAG_V); } }
                { zone(1, "zone-bottom") }
            }

            div { class: "drag-h", onmousedown: move |_| { let _ = eval(js::DRAG_H); } }

            // Right: zone 2
            div { class: "panel-right",
                { zone(2, "zone-grow") }
            }
        }
    }
}
