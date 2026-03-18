use dioxus::prelude::*;

#[component]
pub fn ErrorPane(error: String) -> Element {
    rsx! {
        div { class: "pane-errors",
            if error.is_empty() {
                div { class: "no-err", "no errors" }
            } else {
                div { class: "err-inner", "{error}" }
            }
        }
    }
}
