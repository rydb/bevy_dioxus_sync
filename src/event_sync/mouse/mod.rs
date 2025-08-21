use bevy_input::{mouse::MouseButtonInput, prelude::*, ButtonState};
use bevy_ecs::prelude::*;
use bevy_window::CursorMoved;
use blitz_traits::events::{BlitzMouseButtonEvent, MouseEventButton, MouseEventButtons, UiEvent};
use dioxus::html::Modifiers;
use dioxus_native::DioxusDocument;
use blitz_dom::Document;


use crate::event_sync::does_catch_events;

#[derive(Resource, Default)]
pub struct MouseState {
    pub x: f32,
    pub y: f32,
    pub buttons: MouseEventButtons,
    pub mods: Modifiers,
}

pub(crate) fn handle_mouse_events(
    mut dioxus_doc: NonSendMut<DioxusDocument>,
    mut cursor_moved: EventReader<CursorMoved>,
    mut mouse_button_input_events: ResMut<Events<MouseButtonInput>>,
    mut mouse_buttons: ResMut<ButtonInput<MouseButton>>,
    mut last_mouse_state: Local<MouseState>,
) {
    if cursor_moved.is_empty() && mouse_button_input_events.is_empty() {
        return;
    }

    let mouse_state = &mut last_mouse_state;

    for cursor_event in cursor_moved.read() {
        mouse_state.x = cursor_event.position.x;
        mouse_state.y = cursor_event.position.y;
        dioxus_doc.handle_ui_event(UiEvent::MouseMove(BlitzMouseButtonEvent {
            x: mouse_state.x,
            y: mouse_state.y,
            button: Default::default(),
            buttons: mouse_state.buttons,
            mods: mouse_state.mods,
        }));
    }

    for event in mouse_button_input_events
        .get_cursor()
        .read(&mouse_button_input_events)
    {
        let button_blitz = match event.button {
            MouseButton::Left => MouseEventButton::Main,
            MouseButton::Right => MouseEventButton::Secondary,
            MouseButton::Middle => MouseEventButton::Auxiliary,
            MouseButton::Back => MouseEventButton::Fourth,
            MouseButton::Forward => MouseEventButton::Fifth,
            _ => continue,
        };
        let buttons_blitz = MouseEventButtons::from(button_blitz);
        match event.state {
            ButtonState::Pressed => {
                mouse_state.buttons |= buttons_blitz;
                dioxus_doc.handle_ui_event(UiEvent::MouseDown(BlitzMouseButtonEvent {
                    x: mouse_state.x,
                    y: mouse_state.y,
                    button: button_blitz,
                    buttons: mouse_state.buttons,
                    mods: mouse_state.mods,
                }));
            }
            ButtonState::Released => {
                mouse_state.buttons &= !buttons_blitz;
                dioxus_doc.handle_ui_event(UiEvent::MouseUp(BlitzMouseButtonEvent {
                    x: mouse_state.x,
                    y: mouse_state.y,
                    button: button_blitz,
                    buttons: mouse_state.buttons,
                    mods: mouse_state.mods,
                }));
            }
        }
    }

    let should_catch_events = dioxus_doc
        .hit(mouse_state.x, mouse_state.y)
        .map(|hit| does_catch_events(&dioxus_doc, hit.node_id))
        .unwrap_or(false);
    if should_catch_events {
        mouse_button_input_events.clear();
        mouse_buttons.reset_all();
    }

    dioxus_doc.resolve();
}