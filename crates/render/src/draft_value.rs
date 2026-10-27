//! Draft editing for text inputs that only report committed values.
//!
//! While a field with the class is focused, text changes are applied without
//! telling the app, so a partial or out of range value never reaches it. Enter
//! or blur reports the kept value once, checked against the input's own type,
//! min and max attributes.
//!
//! TODO: this is a stopgap until the DOM layer dispatches submit and change
//! events. Replace this note with the upstream issue link once filed, then
//! delete this module when submit handling lands.

use blitz_traits::events::BlitzKeyEvent;
use blitz_traits::node_id::NodeId;
use dioxus_native::DioxusDocument;
use keyboard_types::{Key, Modifiers, NamedKey};

use crate::text_input::{
    element_has_class, focused_text_input, normalise_text_input, report_text_input_value,
    set_text_input_text, text_input_text,
};

/// CSS class name that opts a text input into draft editing.
pub const DRAFT_VALUE_CLASS: &str = "draft-value";

/// Editing state for an input that keeps its text across renders.
pub struct Draft {
    node: NodeId,
    text: String,
    committed: String,
}

/// The focused draft input, when its edits are kept from the app.
pub fn quiet_target(dioxus_doc: &DioxusDocument) -> Option<NodeId> {
    focused_text_input(dioxus_doc)
        .filter(|node| element_has_class(dioxus_doc, *node, DRAFT_VALUE_CLASS))
}

/// Starts, continues, or finishes the draft for the focused input.
pub fn track(dioxus_doc: &mut DioxusDocument, draft: &mut Option<Draft>) {
    let Some(node) = quiet_target(dioxus_doc) else {
        finish(dioxus_doc, draft);
        return;
    };

    match draft {
        Some(state) if state.node == node => {
            if let Some(text) = text_input_text(dioxus_doc, node) {
                state.text = text;
            }
        }
        _ => {
            let text = text_input_text(dioxus_doc, node).unwrap_or_default();
            *draft = Some(Draft {
                node,
                text: text.clone(),
                committed: text,
            });
        }
    }
}

/// Keeps the typed text when it is valid, otherwise restores the committed text.
pub fn finish(dioxus_doc: &mut DioxusDocument, draft: &mut Option<Draft>) {
    let Some(state) = draft.take() else {
        return;
    };

    // An untouched field has nothing to report, since nothing was typed.
    if state.text == state.committed {
        return;
    }

    let text = normalise_text_input(dioxus_doc, state.node, &state.text).unwrap_or(state.committed);

    // The typed text never reached the app, so the kept text is reported once
    // to bring the app's value in step with what is now on screen.
    report_text_input_value(dioxus_doc, state.node, &text);
}

/// Puts the typed text back after a render, reporting whether it changed.
pub fn restore(dioxus_doc: &mut DioxusDocument, draft: &Option<Draft>) -> bool {
    let Some(state) = draft else {
        return false;
    };

    if text_input_text(dioxus_doc, state.node).as_deref() == Some(state.text.as_str()) {
        return false;
    }

    set_text_input_text(dioxus_doc, state.node, &state.text);
    true
}

/// Whether a key changes the text held by a text input.
pub fn edits_text(event: &BlitzKeyEvent) -> bool {
    if !event.state.is_pressed() {
        return false;
    }

    let action_mod =
        event.modifiers.contains(Modifiers::CONTROL) || event.modifiers.contains(Modifiers::META);

    match &event.key {
        // A newline submits a single line input instead of editing it.
        Key::Character(text) if text == "\n" => false,
        Key::Character(text) => {
            !action_mod || matches!(text.to_lowercase().as_str(), "c" | "x" | "v")
        }
        Key::Named(NamedKey::Backspace) | Key::Named(NamedKey::Delete) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use blitz_traits::events::{BlitzKeyEvent, KeyState};
    use keyboard_types::{Code, Key, Location, Modifiers, NamedKey};

    use super::edits_text;

    fn key(key: Key, modifiers: Modifiers) -> BlitzKeyEvent {
        BlitzKeyEvent {
            key,
            code: Code::Unidentified,
            modifiers,
            location: Location::Standard,
            is_auto_repeating: false,
            is_composing: false,
            state: KeyState::Pressed,
            text: None,
        }
    }

    #[test]
    fn typing_a_character_edits_text() {
        assert!(edits_text(&key(
            Key::Character("5".into()),
            Modifiers::empty()
        )));
    }

    #[test]
    fn deleting_edits_text() {
        assert!(edits_text(&key(
            Key::Named(NamedKey::Backspace),
            Modifiers::empty()
        )));
        assert!(edits_text(&key(
            Key::Named(NamedKey::Delete),
            Modifiers::empty()
        )));
    }

    #[test]
    fn paste_and_cut_edit_text() {
        assert!(edits_text(&key(
            Key::Character("v".into()),
            Modifiers::CONTROL
        )));
        assert!(edits_text(&key(
            Key::Character("x".into()),
            Modifiers::META
        )));
    }

    #[test]
    fn enter_and_navigation_do_not_edit_text() {
        assert!(!edits_text(&key(
            Key::Named(NamedKey::Enter),
            Modifiers::empty()
        )));
        assert!(!edits_text(&key(
            Key::Character("\n".into()),
            Modifiers::empty()
        )));
        assert!(!edits_text(&key(
            Key::Named(NamedKey::ArrowLeft),
            Modifiers::empty()
        )));
        assert!(!edits_text(&key(
            Key::Named(NamedKey::Tab),
            Modifiers::empty()
        )));
        assert!(!edits_text(&key(
            Key::Character("a".into()),
            Modifiers::CONTROL
        )));
    }
}
