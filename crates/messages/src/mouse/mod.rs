use bevy_dioxus_render::worker::VdomThreadRegistry;
use bevy_dioxus_render::{DioxusUiPickFilter, DioxusUiPickState, DioxusUiQuad, DioxusWindowUiQuad};
use bevy_dioxus_tracing::error;
use bevy_ecs::prelude::*;
use bevy_input::{ButtonState, mouse::MouseButtonInput, prelude::*};
use bevy_math::prelude::*;
use bevy_picking::backend::PointerHits;
use bevy_transform::components::GlobalTransform;
use bevy_window::CursorMoved;
use blitz_traits::events::{
    BlitzPointerEvent, BlitzPointerId, MouseEventButton, MouseEventButtons, PointerCoords, UiEvent,
};
use dioxus_html::Modifiers;

#[derive(Resource, Default)]
pub struct MouseState {
    pub x: f32,
    pub y: f32,
    pub buttons: MouseEventButtons,
    pub mods: Modifiers,
}

pub struct UiPickState {
    pub hit_entity: Entity,
    pub local_cords: Vec2,
    pub world_cords: Vec3,
}

/// Holds the per-frame picking result for world-space dioxus UI quads.
#[derive(Resource, Default)]
pub struct WorldSpacePickingState {
    pub pick: Option<UiPickState>,
}

/// Communicates which input space handled cursor events this frame.
#[derive(Resource, Default)]
pub struct MouseMessageRouting {
    pub window_space_handled: bool,
}

pub(crate) fn update_world_space_picking(
    mut pointer_hits: MessageReader<PointerHits>,
    world_quads: Query<(&DioxusUiQuad, &GlobalTransform), Without<DioxusWindowUiQuad>>,
    mut picking_state: ResMut<WorldSpacePickingState>,
) {
    *picking_state = WorldSpacePickingState::default();

    for hits in pointer_hits.read() {
        for (entity, hit_data) in &hits.picks {
            if let Ok((quad, transform)) = world_quads.get(*entity) {
                let Some(world_pos) = hit_data.position else {
                    continue;
                };
                let Some(half) = quad.local_half_extents else {
                    continue;
                };
                let Some(wh) = quad.computed_wh else {
                    continue;
                };

                let local_pos = transform.affine().inverse().transform_point3(world_pos);

                let u = (local_pos.x + half.x) / (2.0 * half.x);
                let v = (local_pos.y + half.y) / (2.0 * half.y);

                let pixel_x = u * wh.x;
                let pixel_y = (1.0 - v) * wh.y;

                picking_state.pick = Some(UiPickState { hit_entity: *entity, local_cords: Vec2::new(pixel_x, pixel_y), world_cords: hit_data.position.unwrap() });
                break;
            }
        }
    }
}

/// Sends cursor-move events to window overlay entities and records whether
/// window space handled input this frame.
pub(crate) fn window_space_mouse_messages(
    mut registry: NonSendMut<VdomThreadRegistry>,
    mut cursor_moved: MessageReader<CursorMoved>,
    mut mouse_state: ResMut<MouseState>,
    mut routing: ResMut<MouseMessageRouting>,
    pick_state: Res<DioxusUiPickState>,
    window_ui: Query<Entity, With<DioxusWindowUiQuad>>,
) {
    routing.window_space_handled = pick_state.active.contains(DioxusUiPickFilter::WINDOW_SPACE);

    if cursor_moved.is_empty() {
        return;
    }

    let Ok(window_ui) = window_ui.single() else {
        error!("This system is only implemented for one window, not multiple. Exiting early. TODO: support more windows. ");
        return;
    };

    for cursor_event in cursor_moved.read() {
        mouse_state.x = cursor_event.position.x;
        mouse_state.y = cursor_event.position.y;

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
            button: Default::default(),
            buttons: mouse_state.buttons,
            mods: mouse_state.mods,
            details: Default::default(),
        };
        
        if let Some(worker) = registry.workers.get(&window_ui) {
            let _ = worker
                .input_tx
                .try_send((window_ui, UiEvent::PointerMove(pointer_event.clone())));
        }
    }
}

/// Sends cursor-move events to the world-space picked entity.
pub(crate) fn world_space_mouse_messages(
    mut registry: NonSendMut<VdomThreadRegistry>,
    mut cursor_moved: MessageReader<CursorMoved>,
    mut mouse_state: ResMut<MouseState>,
    routing: Res<MouseMessageRouting>,
    picking_state: Res<WorldSpacePickingState>,
) {
    if routing.window_space_handled {
        return;
    }

    if cursor_moved.is_empty() {
        return;
    }

    for cursor_event in cursor_moved.read() {
        mouse_state.x = cursor_event.position.x;
        mouse_state.y = cursor_event.position.y;

        if let Some(pick) = &picking_state.pick {
            if let Some(worker) = registry.workers.get_mut(&pick.hit_entity) {
                let local_event = BlitzPointerEvent {
                    id: BlitzPointerId::Mouse,
                    is_primary: true,
                    coords: PointerCoords {
                        page_x: pick.local_cords.x,
                        page_y: pick.local_cords.y,
                        screen_x: pick.local_cords.x,
                        screen_y: pick.local_cords.y,
                        client_x: pick.local_cords.x,
                        client_y: pick.local_cords.y,
                    },
                    button: Default::default(),
                    buttons: mouse_state.buttons,
                    mods: mouse_state.mods,
                    details: Default::default(),
                };
                let _ = worker
                    .input_tx
                    .try_send((pick.hit_entity, UiEvent::PointerMove(local_event)));
            }
        }
    }
}

/// Sends button press and release events, routing based on the active pick space.
pub(crate) fn blitz_mouse_button_handling(
    mut registry: NonSendMut<VdomThreadRegistry>,
    mouse_button_input_events: ResMut<Messages<MouseButtonInput>>,
    mut mouse_state: ResMut<MouseState>,
    pick_state: Res<DioxusUiPickState>,
    picking_state: Res<WorldSpacePickingState>,
    window_ui: Query<Entity, With<DioxusWindowUiQuad>>,
) {
    if mouse_button_input_events.is_empty() {
        return;
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
            }
            ButtonState::Released => {
                mouse_state.buttons &= !buttons_blitz;
            }
        }

        match pick_state.active {
            DioxusUiPickFilter::WINDOW_SPACE => {
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
                let ui_event = match event.state {
                    ButtonState::Pressed => UiEvent::PointerDown(pointer_event),
                    ButtonState::Released => UiEvent::PointerUp(pointer_event),
                };
                if let Ok(window_ui) = window_ui.single() {
                    if let Some(worker) = registry.workers.get(&window_ui) {
                        let _ = worker.input_tx.try_send((window_ui, ui_event));
                    }
                }
            }
            DioxusUiPickFilter::WORLD_SPACE => {
                if let Some(pick) = &picking_state.pick {
                    if let Some(worker) = registry.workers.get_mut(&pick.hit_entity) {
                        let pointer_event = BlitzPointerEvent {
                            id: BlitzPointerId::Mouse,
                            is_primary: true,
                            coords: PointerCoords {
                                page_x: pick.local_cords.x,
                                page_y: pick.local_cords.y,
                                screen_x: pick.local_cords.x,
                                screen_y: pick.local_cords.y,
                                client_x: pick.local_cords.x,
                                client_y: pick.local_cords.y,
                            },
                            button: button_blitz,
                            buttons: mouse_state.buttons,
                            mods: mouse_state.mods,
                            details: Default::default(),
                        };
                        let ui_event = match event.state {
                            ButtonState::Pressed => UiEvent::PointerDown(pointer_event),
                            ButtonState::Released => UiEvent::PointerUp(pointer_event),
                        };
                        let _ = worker
                            .input_tx
                            .try_send((pick.hit_entity, ui_event));
                    }
                }
            }
            _ => {}
        }
    }
}