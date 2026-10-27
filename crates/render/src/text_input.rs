//! Helpers for text inputs driven by CSS classes.

use blitz_dom::Document;
use blitz_dom::local_name;
use blitz_traits::events::{BlitzImeEvent, UiEvent};
use blitz_traits::node_id::NodeId;
use dioxus_native::DioxusDocument;

/// Whether an element carries a class.
pub fn element_has_class(dioxus_doc: &DioxusDocument, node_id: NodeId, class: &str) -> bool {
    let inner = dioxus_doc.inner.borrow();
    inner
        .get_node(node_id)
        .and_then(|node| node.element_data())
        .and_then(|element| element.attr(local_name!("class")))
        .is_some_and(|value| value.split_whitespace().any(|word| word == class))
}

/// The focused node when it is a text input.
pub fn focused_text_input(dioxus_doc: &DioxusDocument) -> Option<NodeId> {
    let inner = dioxus_doc.inner.borrow();
    let node_id = inner.get_focussed_node_id()?;
    inner
        .get_node(node_id)
        .and_then(|node| node.element_data())
        .and_then(|element| element.text_input_data())
        .map(|_| node_id)
}

/// Text currently held by a text input.
pub fn text_input_text(dioxus_doc: &DioxusDocument, node_id: NodeId) -> Option<String> {
    let inner = dioxus_doc.inner.borrow();
    let data = inner
        .get_node(node_id)
        .and_then(|node| node.element_data())
        .and_then(|element| element.text_input_data())?;
    Some(data.editor.raw_text().to_string())
}

/// Replaces a text input's text, leaving the caret at the end.
pub fn set_text_input_text(dioxus_doc: &mut DioxusDocument, node_id: NodeId, text: &str) {
    dioxus_doc
        .inner
        .borrow_mut()
        .with_text_input(node_id, |mut driver| {
            driver.select_all();
            driver.insert_or_replace_selection(text);
        });
}

/// Selects the whole value of a text input.
pub fn select_all_text(dioxus_doc: &mut DioxusDocument, node_id: NodeId) {
    dioxus_doc
        .inner
        .borrow_mut()
        .with_text_input(node_id, |mut driver| driver.select_all());
}

/// Replaces a text input's value and reports it to the app.
pub fn report_text_input_value(dioxus_doc: &mut DioxusDocument, node_id: NodeId, text: &str) {
    // A removed input cannot take focus, so nothing can be committed into it.
    if text_input_text(dioxus_doc, node_id).is_none() {
        return;
    }

    let previous_focus = {
        let mut inner = dioxus_doc.inner.borrow_mut();
        let previous = inner.get_focussed_node_id();
        if previous != Some(node_id) {
            inner.set_focus_to(node_id);
        }
        previous
    };

    // Committing applies to the focused input, so the target must hold focus.
    if dioxus_doc.inner.borrow().get_focussed_node_id() != Some(node_id) {
        return;
    }

    // The commit replaces the selection, which reports the new value as well.
    select_all_text(dioxus_doc, node_id);
    dioxus_doc.handle_ui_event(UiEvent::Ime(BlitzImeEvent::Commit(text.to_string())));

    if previous_focus != Some(node_id) {
        if let Some(previous) = previous_focus {
            dioxus_doc.inner.borrow_mut().set_focus_to(previous);
        }
    }
}

/// Validates edited text against the input's attributes.
pub fn normalise_text_input(
    dioxus_doc: &DioxusDocument,
    node_id: NodeId,
    text: &str,
) -> Option<String> {
    let (input_type, min, max) = {
        let inner = dioxus_doc.inner.borrow();
        let element = inner
            .get_node(node_id)
            .and_then(|node| node.element_data())?;
        (
            element
                .attr(local_name!("type"))
                .unwrap_or("text")
                .to_string(),
            element
                .attr(local_name!("min"))
                .and_then(|value| value.parse::<f64>().ok()),
            element
                .attr(local_name!("max"))
                .and_then(|value| value.parse::<f64>().ok()),
        )
    };

    normalise_text(&input_type, min, max, text)
}

/// Validates text against an input type and range.
pub fn normalise_text(
    input_type: &str,
    min: Option<f64>,
    max: Option<f64>,
    text: &str,
) -> Option<String> {
    if input_type != "number" {
        return Some(text.to_string());
    }

    let mut value: f64 = text.trim().parse().ok()?;
    if !value.is_finite() {
        return None;
    }

    if let Some(min) = min {
        value = value.max(min);
    }
    if let Some(max) = max {
        value = value.min(max);
    }

    Some(format!("{value}"))
}

#[cfg(test)]
mod tests {
    use super::normalise_text;

    #[test]
    fn accepts_a_number() {
        assert_eq!(
            normalise_text("number", None, None, "3.5"),
            Some("3.5".into())
        );
    }

    #[test]
    fn clamps_to_the_range() {
        assert_eq!(
            normalise_text("number", Some(0.0), Some(10.0), "-4"),
            Some("0".into())
        );
        assert_eq!(
            normalise_text("number", Some(0.0), Some(10.0), "99"),
            Some("10".into())
        );
        assert_eq!(
            normalise_text("number", Some(0.0), None, "-2"),
            Some("0".into())
        );
    }

    #[test]
    fn rejects_text_that_is_not_a_number() {
        assert_eq!(normalise_text("number", None, None, "12a"), None);
        assert_eq!(normalise_text("number", None, None, ""), None);
        assert_eq!(normalise_text("number", None, None, "NaN"), None);
    }

    #[test]
    fn leaves_other_types_alone() {
        assert_eq!(
            normalise_text("text", None, None, "12a"),
            Some("12a".into())
        );
    }
}
