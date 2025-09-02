use dioxus_native::DioxusDocument;

pub mod keyboard;
pub mod mouse;
pub mod plugins;
pub mod window;

const CATCH_EVENTS_CLASS: &str = "catch-events";

fn does_catch_events(dioxus_doc: &DioxusDocument, node_id: usize) -> bool {
    if let Some(node) = dioxus_doc.get_node(node_id) {
        let class = node.attr(blitz_dom::local_name!("class")).unwrap_or("");
        if class
            .split_whitespace()
            .any(|word| word == CATCH_EVENTS_CLASS)
        {
            true
        } else if let Some(parent) = node.parent {
            does_catch_events(dioxus_doc, parent)
        } else {
            false
        }
    } else {
        false
    }
}
