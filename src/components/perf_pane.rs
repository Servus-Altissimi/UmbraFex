use dioxus::prelude::*;
use crate::gpu::PerfStats;

#[component]
pub fn PerfPane(stats: PerfStats) -> Element {
    rsx! {
        div { class: "pane-perf",
            div { class: "perf-section", "performance" }
            div { class: "perf-row",
                span { class: "perf-label", "FPS" }
                span { class: "perf-val perf-fps", "{stats.fps:.1}" }
            }
            div { class: "perf-row",
                span { class: "perf-label", "Frame" }
                span { class: "perf-val", "{stats.frame_ms:.2} ms" }
            }
            div { class: "perf-row",
                span { class: "perf-label", "Size" }
                span { class: "perf-val", "{stats.w} × {stats.h}" }
            }
            div { class: "perf-section", "hardware" }
            div { class: "perf-row",
                span { class: "perf-label", "GPU" }
                span { class: "perf-val", "{stats.gpu_name}" }
            }
            div { class: "perf-row",
                span { class: "perf-label", "Backend" }
                span { class: "perf-val", "{stats.backend}" }
            }
        }
    }
}

