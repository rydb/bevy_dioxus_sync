use bevy_dioxus_render::worker::VdomThreadRegistry;
use bevy_dioxus_render::{DioxusUiQuad, DioxusWindowUiQuad};
use bevy_ecs::prelude::*;
use bevy_input::{ButtonState, mouse::MouseButtonInput, prelude::*};
use bevy_math::Vec2;
use bevy_picking::backend::PointerHits;
use bevy_transform::components::GlobalTransform;
use bevy_window::CursorMoved;
use blitz_traits::events::{
    BlitzPointerEvent, BlitzPointerId, MouseEventButton, MouseEventButtons,
    PointerCoords, UiEvent,
};
use dioxus_html::Modifiers;

#[derive(Resource, Default)]
pub struct MouseState {
    pub x: f32,
    pub y: f32,
    pub buttons: MouseEventButtons,
    pub mods: Modifiers,
}

/// Holds the per-frame picking result for world-space dioxus UI quads.
#[derive(Resource, Default)]
pub struct WorldSpacePickingState {
    pub hit_entity: Option<Entity>,
    pub local_coords: Option<Vec2>,
}

pub(crate) fn update_world_space_picking(
    mut pointer_hits: MessageReader<PointerHits>,
    world_quads: Query<
        (&DioxusUiQuad, &GlobalTransform),
        Without<DioxusWindowUiQuad>,
    >,
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

                let local_pos = transform
                    .affine()
                    .inverse()
                    .transform_point3(world_pos);

                let u = (local_pos.x + half.x) / (2.0 * half.x);
                let v = (local_pos.y + half.y) / (2.0 * half.y);

                let pixel_x = u * wh.x;
                let pixel_y = (1.0 - v) * wh.y;

                picking_state.hit_entity = Some(*entity);
                picking_state.local_coords =
                    Some(Vec2::new(pixel_x, pixel_y));
                break;
            }
        }
    }
}

pub(crate) fn handle_mouse_messages(
    mut registry: NonSendMut<VdomThreadRegistry>,
    mut cursor_moved: MessageReader<CursorMoved>,
    mut mouse_button_input_events: ResMut<Messages<MouseButtonInput>>,
    mut mouse_buttons: ResMut<ButtonInput<MouseButton>>,
    mut last_mouse_state: Local<MouseState>,
    picking_state: Res<WorldSpacePickingState>,
    window_overlay_query: Query<Entity, With<DioxusWindowUiQuad>>,
) {
    if cursor_moved.is_empty() && mouse_button_input_events.is_empty() {
        return;
    }

    let mut should_catch_events = false;
    let mouse_state = &mut last_mouse_state;

    let window_overlay_entities: std::collections::HashSet<Entity> =
        window_overlay_query.iter().collect();

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

        for (&entity, worker) in &mut registry.workers {
            if !window_overlay_entities.contains(&entity) {
                continue;
            }
            let _ = worker.input_tx.try_send((
                entity,
                UiEvent::PointerMove(pointer_event.clone()),
            ));
            should_catch_events = true;
        }

        if let (Some(hit_entity), Some(local_coords)) =
            (picking_state.hit_entity, picking_state.local_coords)
        {
            if let Some(worker) = registry.workers.get_mut(&hit_entity) {
                let local_event = BlitzPointerEvent {
                    id: BlitzPointerId::Mouse,
                    is_primary: true,
                    coords: PointerCoords {
                        page_x: local_coords.x,
                        page_y: local_coords.y,
                        screen_x: local_coords.x,
                        screen_y: local_coords.y,
                        client_x: local_coords.x,
                        client_y: local_coords.y,
                    },
                    button: Default::default(),
                    buttons: mouse_state.buttons,
                    mods: mouse_state.mods,
                    details: Default::default(),
                };
                let _ = worker.input_tx.try_send((
                    hit_entity,
                    UiEvent::PointerMove(local_event),
                ));
                should_catch_events = true;
            }
        }
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

        let overlay_coords = PointerCoords {
            page_x: mouse_state.x,
            page_y: mouse_state.y,
            screen_x: mouse_state.x,
            screen_y: mouse_state.y,
            client_x: mouse_state.x,
            client_y: mouse_state.y,
        };

        match event.state {
            ButtonState::Pressed => {
                mouse_state.buttons |= buttons_blitz;

                let pointer_event = BlitzPointerEvent {
                    id: BlitzPointerId::Mouse,
                    is_primary: true,
                    coords: overlay_coords,
                    button: button_blitz,
                    buttons: mouse_state.buttons,
                    mods: mouse_state.mods,
                    details: Default::default(),
                };

                for (&entity, worker) in &mut registry.workers {
                    if !window_overlay_entities.contains(&entity) {
                        continue;
                    }
                    let _ = worker.input_tx.try_send((
                        entity,
                        UiEvent::PointerDown(pointer_event.clone()),
                    ));
                    should_catch_events = true;
                }

                if let (Some(hit_entity), Some(local_coords)) =
                    (picking_state.hit_entity, picking_state.local_coords)
                {
                    if let Some(worker) =
                        registry.workers.get_mut(&hit_entity)
                    {
                        let local_event = BlitzPointerEvent {
                            id: BlitzPointerId::Mouse,
                            is_primary: true,
                            coords: PointerCoords {
                                page_x: local_coords.x,
                                page_y: local_coords.y,
                                screen_x: local_coords.x,
                                screen_y: local_coords.y,
                                client_x: local_coords.x,
                                client_y: local_coords.y,
                            },
                            button: button_blitz,
                            buttons: mouse_state.buttons,
                            mods: mouse_state.mods,
                            details: Default::default(),
                        };
                        let _ = worker.input_tx.try_send((
                            hit_entity,
                            UiEvent::PointerDown(local_event),
                        ));
                        should_catch_events = true;
                    }
                }
            }
            ButtonState::Released => {
                mouse_state.buttons &= !buttons_blitz;

                let pointer_event = BlitzPointerEvent {
                    id: BlitzPointerId::Mouse,
                    is_primary: true,
                    coords: overlay_coords,
                    button: button_blitz,
                    buttons: mouse_state.buttons,
                    mods: mouse_state.mods,
                    details: Default::default(),
                };

                for (&entity, worker) in &mut registry.workers {
                    if !window_overlay_entities.contains(&entity) {
                        continue;
                    }
                    let _ = worker.input_tx.try_send((
                        entity,
                        UiEvent::PointerUp(pointer_event.clone()),
                    ));
                    should_catch_events = true;
                }

                if let (Some(hit_entity), Some(local_coords)) =
                    (picking_state.hit_entity, picking_state.local_coords)
                {
                    if let Some(worker) =
                        registry.workers.get_mut(&hit_entity)
                    {
                        let local_event = BlitzPointerEvent {
                            id: BlitzPointerId::Mouse,
                            is_primary: true,
                            coords: PointerCoords {
                                page_x: local_coords.x,
                                page_y: local_coords.y,
                                screen_x: local_coords.x,
                                screen_y: local_coords.y,
                                client_x: local_coords.x,
                                client_y: local_coords.y,
                            },
                            button: button_blitz,
                            buttons: mouse_state.buttons,
                            mods: mouse_state.mods,
                            details: Default::default(),
                        };
                        let _ = worker.input_tx.try_send((
                            hit_entity,
                            UiEvent::PointerUp(local_event),
                        ));
                        should_catch_events = true;
                    }
                }
            }
        }
    }

    if should_catch_events {
        mouse_button_input_events.clear();
        mouse_buttons.reset_all();
    }
}
