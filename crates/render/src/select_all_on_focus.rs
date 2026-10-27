//! Selects a text input's value when the pointer focuses it.

use blitz_traits::node_id::NodeId;
use dioxus_native::DioxusDocument;

use crate::text_input::{element_has_class, focused_text_input, select_all_text};

/// CSS class name that opts a text input into select all on focus.
pub const SELECT_ALL_ON_FOCUS_CLASS: &str = "select-all-on-focus";

/// Selects the whole value of a focused text input that opted in by class.
///
/// Returns the input whose value was selected.
pub fn select_all_on_focus(dioxus_doc: &mut DioxusDocument) -> Option<NodeId> {
    let node_id = focused_text_input(dioxus_doc)?;

    if !element_has_class(dioxus_doc, node_id, SELECT_ALL_ON_FOCUS_CLASS) {
        return None;
    }

    select_all_text(dioxus_doc, node_id);
    Some(node_id)
}
