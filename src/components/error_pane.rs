use dioxus::prelude::*;

#[component]
pub fn ErrorPane(
    error: String,
    on_run: EventHandler<MouseEvent>,
    on_fullscreen: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "pane-errors", 
            div { class: "bar",
                button { onclick: move |e| on_run.call(e), "Run" }
                button { onclick: move |e| on_fullscreen.call(e), "Fullscreen" }
            }
            if error.is_empty() {
                div { class: "no-err", "no errors" }
            } else {
                div { class: "err-inner", "{error}" }
            }
        }
    }
}
