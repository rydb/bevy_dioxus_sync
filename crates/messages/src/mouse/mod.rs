use bevy_dioxus_interop::DioxusDocuments;
use bevy_dioxus_render::{DioxusUiQuad, DioxusWindowUiQuad};
use bevy_ecs::prelude::*;
use bevy_input::{ButtonState, mouse::MouseButtonInput, prelude::*};
use bevy_math::Vec2;
use bevy_picking::backend::PointerHits;
use bevy_transform::components::GlobalTransform;
use bevy_window::CursorMoved;
use blitz_dom::Document;
use blitz_traits::events::{
    BlitzPointerEvent, BlitzPointerId, MouseEventButton, MouseEventButtons, PointerCoords, UiEvent,
};
use dioxus_html::Modifiers;

use super::does_catch_events;

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
    /// The entity of the world-space quad currently under the cursor.
    pub hit_entity: Option<Entity>,
    /// Local pixel coordinates within the quad's texture.
    pub local_coords: Option<Vec2>,
}

/// Reads [`PointerHits`] from bevy_picking and resolves the nearest world-space dioxus UI hit.
///
/// This system must run after [`bevy_picking::PickingSystems::Backend`].
pub(crate) fn update_world_space_picking(
    mut pointer_hits: MessageReader<PointerHits>,
    world_quads: Query<(&DioxusUiQuad, &GlobalTransform), Without<DioxusWindowUiQuad>>,
    mut picking_state: ResMut<WorldSpacePickingState>,
) {
    // Reset each frame; we will recompute.
    *picking_state = WorldSpacePickingState::default();

    for hits in pointer_hits.read() {
        // Picks are reported per-pointer. Find the first (nearest) hit on a world-space quad.
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

                // Transform world-space hit point into the quad's local space.
                let local_pos = transform.affine().inverse().transform_point3(world_pos);

                // Compute UV coordinates from local position and half-extents.
                // The quad spans local coords [-half.x, half.x] in X and [-half.y, half.y] in Y.
                let u = (local_pos.x + half.x) / (2.0 * half.x);
                let v = (local_pos.y + half.y) / (2.0 * half.y);

                // Convert UV to pixel coordinates within the texture.
                // Flip V because texture Y increases downward.
                let pixel_x = u * wh.x;
                let pixel_y = (1.0 - v) * wh.y;

                picking_state.hit_entity = Some(*entity);
                picking_state.local_coords = Some(Vec2::new(pixel_x, pixel_y));
                break; // Only the nearest hit matters for this pointer.
            }
        }
    }
}

pub(crate) fn handle_mouse_messages(
    mut dioxus_docs: NonSendMut<DioxusDocuments>,
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

    // Collect the set of window overlay entities once.
    let window_overlay_entities: std::collections::HashSet<Entity> =
        window_overlay_query.iter().collect();

    // --- Track cursor position from CursorMoved events ---
    for cursor_event in cursor_moved.read() {
        mouse_state.x = cursor_event.position.x;
        mouse_state.y = cursor_event.position.y;

        // Forward PointerMove to window overlay documents with raw screen coords.
        for (&entity, info) in &mut dioxus_docs.0 {
            if !window_overlay_entities.contains(&entity) {
                continue;
            }
            info.document.handle_ui_event(UiEvent::PointerMove(BlitzPointerEvent {
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

        // Forward PointerMove to the hit world-space quad with local pixel coords.
        if let (Some(hit_entity), Some(local_coords)) =
            (picking_state.hit_entity, picking_state.local_coords)
        {
            if let Some(info) = dioxus_docs.0.get_mut(&hit_entity) {
                info.document.handle_ui_event(UiEvent::PointerMove(BlitzPointerEvent {
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
                }));
            }
        }
    }

    // mouse button events
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

                // Window overlay docs.
                for (&entity, info) in &mut dioxus_docs.0 {
                    if !window_overlay_entities.contains(&entity) {
                        continue;
                    }
                    info.document
                        .handle_ui_event(UiEvent::PointerDown(pointer_event.clone()));
                }

                // Hit world-space doc.
                if let (Some(hit_entity), Some(local_coords)) =
                    (picking_state.hit_entity, picking_state.local_coords)
                {
                    if let Some(info) = dioxus_docs.0.get_mut(&hit_entity) {
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
                        info.document
                            .handle_ui_event(UiEvent::PointerDown(local_event));
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

                // Window overlay docs.
                for (&entity, info) in &mut dioxus_docs.0 {
                    if !window_overlay_entities.contains(&entity) {
                        continue;
                    }
                    info.document
                        .handle_ui_event(UiEvent::PointerUp(pointer_event.clone()));
                }

                // Hit world-space doc.
                if let (Some(hit_entity), Some(local_coords)) =
                    (picking_state.hit_entity, picking_state.local_coords)
                {
                    if let Some(info) = dioxus_docs.0.get_mut(&hit_entity) {
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
                        info.document
                            .handle_ui_event(UiEvent::PointerUp(local_event));
                    }
                }
            }
        }

        // Check window overlay docs.
        for (&entity, info) in &dioxus_docs.0 {
            if !window_overlay_entities.contains(&entity) {
                continue;
            }
            let flip_catch = info
                .document
                .inner
                .borrow()
                .hit(mouse_state.x, mouse_state.y)
                .map(|hit| does_catch_events(&info.document, hit.node_id))
                .unwrap_or(false);
            if flip_catch {
                should_catch_events = true;
            }
        }

        // Check the hit world-space doc.
        if let (Some(hit_entity), Some(local_coords)) =
            (picking_state.hit_entity, picking_state.local_coords)
        {
            if let Some(info) = dioxus_docs.0.get(&hit_entity) {
                let flip_catch = info
                    .document
                    .inner
                    .borrow()
                    .hit(local_coords.x, local_coords.y)
                    .map(|hit| does_catch_events(&info.document, hit.node_id))
                    .unwrap_or(false);
                if flip_catch {
                    should_catch_events = true;
                }
            }
        }
    }

    if should_catch_events {
        mouse_button_input_events.clear();
        mouse_buttons.reset_all();
    }
}
