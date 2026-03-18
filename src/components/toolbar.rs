use dioxus::prelude::*;

#[component]
pub fn Toolbar(
    on_run: EventHandler<MouseEvent>,
    on_fullscreen: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "bar",
            button { onclick: move |e| on_run.call(e), "Run" }
            button { onclick: move |e| on_fullscreen.call(e), "Fullscreen" }
        }
    }
}
