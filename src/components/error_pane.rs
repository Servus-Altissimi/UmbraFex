use dioxus::prelude::*;

#[component]
pub fn ErrorPane(
    error:              String,
    on_run:             EventHandler<MouseEvent>,
    on_fullscreen:      EventHandler<MouseEvent>,
    timeline_enabled:   bool,
    on_timeline_toggle: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "pane-errors", 
            div { class: "bar",
                button { onclick: move |e| on_run.call(e), "Run" }
                button { onclick: move |e| on_fullscreen.call(e), "Fullscreen" }
               
                button {
                    class: if timeline_enabled { "btn-timeline-active" } else { "" },
                    onclick: move |e| on_timeline_toggle.call(e),
                    if timeline_enabled { "Timeline ✓" } else { "Timeline" }
                }
            }
            if error.is_empty() {
                div { class: "no-err", "no errors" }
            } else {
                div { class: "err-inner", "{error}" }
            }
        }
    }
}
