use bevy_dioxus_interop::DioxusDocuments;
use bevy_ecs::prelude::*;
use bevy_input::{ButtonState, mouse::MouseButtonInput, prelude::*};
use bevy_window::CursorMoved;
use blitz_dom::Document;
use blitz_traits::events::{BlitzPointerEvent, BlitzPointerId, MouseEventButton, MouseEventButtons, PointerCoords, UiEvent};
use dioxus_html::Modifiers;

use super::does_catch_events;

#[derive(Resource, Default)]
pub struct MouseState {
    pub x: f32,
    pub y: f32,
    pub buttons: MouseEventButtons,
    pub mods: Modifiers,
}

pub(crate) fn handle_mouse_messages(
    mut dioxus_docs: NonSendMut<DioxusDocuments>,
    mut cursor_moved: MessageReader<CursorMoved>,
    mut mouse_button_input_events: ResMut<Messages<MouseButtonInput>>,
    mut mouse_buttons: ResMut<ButtonInput<MouseButton>>,
    mut last_mouse_state: Local<MouseState>,
) {
    if cursor_moved.is_empty() && mouse_button_input_events.is_empty() {
        return;
    }
    let mut should_catch_events = false;

    let mouse_state = &mut last_mouse_state;

    for cursor_event in cursor_moved.read() {
        mouse_state.x = cursor_event.position.x;
        mouse_state.y = cursor_event.position.y;
        for (_entiy, dioxus_doc) in &mut dioxus_docs.0 {
            dioxus_doc.handle_ui_event(UiEvent::PointerMove(BlitzPointerEvent {
                id: BlitzPointerId::Mouse,
                is_primary: true,
                coords: PointerCoords {
                    page_x: mouse_state.x,
                    page_y: mouse_state.y,
                    screen_x: mouse_state.x,
                    screen_y: mouse_state.y,
                    client_x: mouse_state.x,
                    client_y: mouse_state.y,
                },
                button: Default::default(),
                buttons: mouse_state.buttons,
                mods: mouse_state.mods,
                details: Default::default(),
            }));
        }
    }

    for event in mouse_button_input_events
        .get_cursor()
        .read(&mouse_button_input_events)
    {
        for (_entity, dioxus_doc) in &mut dioxus_docs.0 {
            let button_blitz = match event.button {
                MouseButton::Left => MouseEventButton::Main,
                MouseButton::Right => MouseEventButton::Secondary,
                MouseButton::Middle => MouseEventButton::Auxiliary,
                MouseButton::Back => MouseEventButton::Fourth,
                MouseButton::Forward => MouseEventButton::Fifth,
                _ => continue,
            };
            let buttons_blitz = MouseEventButtons::from(button_blitz);
            let pointer_event = BlitzPointerEvent {
                id: BlitzPointerId::Mouse,
                is_primary: true,
                coords: PointerCoords {
                    page_x: mouse_state.x,
                    page_y: mouse_state.y,
                    screen_x: mouse_state.x,
                    screen_y: mouse_state.y,
                    client_x: mouse_state.x,
                    client_y: mouse_state.y,
                },
                button: button_blitz,
                buttons: mouse_state.buttons,
                mods: mouse_state.mods,
                details: Default::default(),
            };
            match event.state {
                ButtonState::Pressed => {
                    mouse_state.buttons |= buttons_blitz;
                    dioxus_doc.handle_ui_event(UiEvent::PointerDown(pointer_event));
                }
                ButtonState::Released => {
                    mouse_state.buttons &= !buttons_blitz;
                    dioxus_doc.handle_ui_event(UiEvent::PointerUp(pointer_event));
                }
            }
            let flip_catch_events = dioxus_doc
                .inner.borrow()
                .hit(mouse_state.x, mouse_state.y)
                .map(|hit| does_catch_events(&dioxus_doc, hit.node_id))
                .unwrap_or(false);

            if flip_catch_events {
                should_catch_events = true;
            }
        }
    }

    if should_catch_events {
        mouse_button_input_events.clear();
        mouse_buttons.reset_all();
    }
}
