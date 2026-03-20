use dioxus::prelude::*;

#[component]
pub fn TimelinePane( 
    duration:           f32,
    position:           f32, // Current playhead position in seconds (fed back from the render coroutine)
    playing:            bool, 
    on_duration_change: EventHandler<f32>, 
    on_seek:            EventHandler<f32>, 
    on_play_pause:      EventHandler<bool>,
) -> Element {

    // Buffer the typed duration text in a local signal so it is owned by this component.
    let mut duration_str = use_hook(|| Signal::new(format!("{:.1}", duration)));
    rsx! {
        div { class: "pane-timeline",

            // Row 1: play/pause button + current time readout
            div { class: "timeline-controls",
                button {
                    class: "timeline-btn",
                    onclick: move |_| on_play_pause.call(!playing), 
                    if playing { "⏸ Pause" } else { "▶ Play" }
                }
                span { class: "timeline-time-display",
                    "{position:.2}s \u{00a0}/\u{00a0} {duration:.1}s" // non-breaking spaces around the slash for readability
                }
            }

            // Row 2: scrubber
            div { class: "timeline-scrubber-wrap",
                input {
                    r#type: "range",
                    class:  "timeline-scrubber",
                    min:    "0.0",
                    max:    "{duration}", 
                    step:   "0.001",
                    value:  "{position}",
                    oninput: move |e| {
                        if let Ok(v) = e.value().parse::<f32>() {
                            on_seek.call(v);
                        }
                    },
                }
            }

            // Row 3: duration control
            div { class: "timeline-duration-row",
                span { class: "timeline-label", "Duration" }
                input {
                    r#type:   "number",
                    class:    "timeline-duration-input",
                    min:      "0.1",
                    max:      "3600",
                    step:     "0.5",
                    // Bound to the LOCAL signal, not the prop. The local signal is only mutated by oninput below 
                    value:    "{duration_str}",
                    oninput: move |e| { 
                        duration_str.set(e.value());
                    }, 
                    onchange: move |e| {
                        if let Ok(v) = e.value().parse::<f32>() {
                            let clamped = v.max(0.1);
                            // Normalise the display string to match what the parent will pass back as the "duration" prop
                            duration_str.set(format!("{clamped:.1}"));
                            on_duration_change.call(clamped);
                        }
                    },
                }
                span { class: "timeline-label", "s" }
            }
        }
    }
}
