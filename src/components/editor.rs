use dioxus::document::eval;
use dioxus::prelude::*;

use crate::js;

#[component]
pub fn Editor(
    src: String,
    highlighted: String,
    on_input: EventHandler<String>,
) -> Element {
    let line_count = src.lines().count().max(1);

    rsx! {
        div { class: "editor-wrap",
            div { class: "gutter",
                for n in 1..=line_count {
                    div { class: "gutter-line", "{n}" }
                }
            }
           
            div { class: "editor-content",
                // highlight-overlay sits beneath the transparent textarea, its scroll position is kept in sync via SYNC_SCROLL_JS.
                div {
                    class: "highlight-overlay",
                    dangerous_inner_html: "{highlighted}",
                }
                textarea {
                    class: "code",
                    spellcheck: false,
                    value: "{src}",
                    oninput: move |e| on_input.call(e.value()),
                    onscroll: move |_| { let _ = eval(js::SYNC_SCROLL); },
                }
            }
        }
    }
}
