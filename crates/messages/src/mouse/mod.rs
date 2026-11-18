use bevy_camera::Camera;
use bevy_dioxus_render::layout::DioxusPointerSender;
use bevy_dioxus_render::slots::{CapturedPointer, Vec2Px};
use bevy_dioxus_render::worker::VdomThreadRegistry;
use bevy_dioxus_render::{
    DioxusUiHitState, DioxusUiPickFilter, DioxusUiPickState, DioxusUiQuad, DioxusWindowUiQuad,
};
use bevy_dioxus_tracing::error;
use bevy_ecs::prelude::*;
use bevy_input::{
    ButtonState,
    mouse::{MouseButtonInput, MouseScrollUnit, MouseWheel},
    prelude::*,
};
use bevy_math::prelude::*;
use bevy_picking::backend::PointerHits;
use bevy_shape::InfinitePlane3d;
use bevy_transform::components::GlobalTransform;
use bevy_window::{CursorMoved, Window};
use blitz_traits::events::{
    BlitzPointerEvent, BlitzPointerId, BlitzWheelDelta, BlitzWheelEvent, MouseEventButton,
    MouseEventButtons, PointerCoords, UiEvent,
};
use dioxus_html::Modifiers;

/// Pointer position and held button state shared by the input routing systems.
#[derive(Resource, Default)]
pub struct MouseState {
    pub x: f32,
    pub y: f32,
    pub buttons: MouseEventButtons,
    pub mods: Modifiers,
    /// Document that owns the current pointer gesture, if a button is held.
    pub gesture: Option<GestureTarget>,
    /// Document that received the last press, used to route keyboard input.
    pub focused: Option<Entity>,
}

/// Document latched for the duration of a pointer gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GestureTarget {
    /// Entity owning the document that received the press.
    pub entity: Entity,
    /// Space the press was routed to.
    pub space: DioxusUiPickFilter,
    /// Live pointer position inside the document, in surface pixels.
    pub local: Vec2,
    /// Camera that produced the press for a world-space surface.
    pub camera: Option<Entity>,
}

pub struct UiPickState {
    pub hit_entity: Entity,
    pub local_cords: Vec2,
    pub world_cords: Vec3,
    pub camera: Entity,
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

/// Multiple of the quad half extents that grazing rays are clamped to.
const GRAZE_LIMIT: f32 = 4.0;

/// Maps a point in quad local space to surface pixels.
pub(crate) fn quad_surface_pixels(local: Vec3, half: Vec2, surface: Vec2) -> Vec2 {
    if half.x <= f32::EPSILON || half.y <= f32::EPSILON {
        return Vec2::ZERO;
    }
    let u = (local.x + half.x) / (2.0 * half.x);
    let v = (local.y + half.y) / (2.0 * half.y);
    Vec2::new(u * surface.x, (1.0 - v) * surface.y)
}

/// Maps a window cursor position into surface pixels.
pub fn window_pointer_local(cursor: Vec2, window: Vec2, quad_pixels: Vec2) -> Vec2 {
    let width = window.x.max(1.0);
    let height = window.y.max(1.0);
    Vec2::new(
        cursor.x / width * quad_pixels.x,
        cursor.y / height * quad_pixels.y,
    )
}

/// Maps a physical cursor position into surface pixels on a world-space quad.
///
/// The cursor ray intersects the quad plane, so points outside the quad keep mapping.
pub fn world_pointer_local(
    cursor_physical: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    quad: &DioxusUiQuad,
    quad_transform: &GlobalTransform,
) -> Option<Vec2> {
    let half = quad.local_half_extents?;
    let surface = quad.computed_wh?;
    if half.x <= f32::EPSILON || half.y <= f32::EPSILON {
        return None;
    }

    // The ray is built from the same logical position that picking uses.
    let scale = camera
        .target_scaling_factor()
        .unwrap_or(1.0)
        .max(f32::EPSILON);
    let ray = camera
        .viewport_to_world(camera_transform, cursor_physical / scale)
        .ok()?;

    let normal = quad_transform.back();
    let distance =
        ray.intersect_plane(quad_transform.translation(), InfinitePlane3d::new(normal))?;
    if !distance.is_finite() || distance <= 0.0 {
        return None;
    }

    let world = ray.get_point(distance);
    let local = quad_transform.affine().inverse().transform_point3(world);
    if !local.is_finite() {
        return None;
    }

    // A grazing ray leaves the plane near parallel, so the point is bounded.
    let limit = half * GRAZE_LIMIT;
    let bounded = Vec3::new(
        local.x.clamp(-limit.x, limit.x),
        local.y.clamp(-limit.y, limit.y),
        local.z,
    );
    Some(quad_surface_pixels(bounded, half, surface))
}

/// Resolves the owning pick space from the reported hit tests and the live pick.
pub(crate) fn resolve_pick_space(
    hits: Res<DioxusUiHitState>,
    picking_state: Res<WorldSpacePickingState>,
    mouse_state: Res<MouseState>,
    mut pick_state: ResMut<DioxusUiPickState>,
) {
    // A latched gesture keeps the space that received the press.
    if mouse_state.gesture.is_some() {
        return;
    }

    let active = if hits.window_space_caught {
        DioxusUiPickFilter::WINDOW_SPACE
    } else if hits.world_space_caught && picking_state.pick.is_some() {
        DioxusUiPickFilter::WORLD_SPACE
    } else {
        DioxusUiPickFilter::NONE
    };

    if pick_state.active != active {
        pick_state.active = active;
    }
}

pub(crate) fn update_world_space_picking(
    mut pointer_hits: MessageReader<PointerHits>,
    world_quads: Query<(&DioxusUiQuad, &GlobalTransform), Without<DioxusWindowUiQuad>>,
    mut picking_state: ResMut<WorldSpacePickingState>,
) {
    let mut pick = None;
    let mut reported = false;

    // Every backend writes one message per camera, so the frame is coalesced
    // into one result before the pick is replaced.
    for hits in pointer_hits.read() {
        reported = true;

        if pick.is_some() {
            continue;
        }

        for (entity, hit_data) in &hits.picks {
            let Ok((quad, transform)) = world_quads.get(*entity) else {
                continue;
            };
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

            pick = Some(UiPickState {
                hit_entity: *entity,
                local_cords: quad_surface_pixels(local_pos, half, wh),
                world_cords: world_pos,
                camera: hit_data.camera,
            });
            break;
        }
    }

    // Without pointer messages the pointer did not move, so the pick stands.
    if pick.is_some() {
        picking_state.pick = pick;
    } else if reported {
        picking_state.pick = None;
    }
}

/// Sends cursor-move events to window overlay entities and records whether
/// window space handled input this frame.
pub(crate) fn window_space_mouse_messages(
    registry: NonSendMut<VdomThreadRegistry>,
    mut cursor_moved: MessageReader<CursorMoved>,
    mut mouse_state: ResMut<MouseState>,
    mut routing: ResMut<MouseMessageRouting>,
    pick_state: Res<DioxusUiPickState>,
    window_ui: Query<(Entity, &DioxusUiQuad), With<DioxusWindowUiQuad>>,
    windows: Query<&Window>,
) {
    routing.window_space_handled = pick_state.active.contains(DioxusUiPickFilter::WINDOW_SPACE);

    // A latched gesture owns the pointer until every button is released.
    if mouse_state.gesture.is_some() {
        return;
    }

    if cursor_moved.is_empty() {
        return;
    }

    let Ok((window_ui, quad)) = window_ui.single() else {
        error!(
            "This system is only implemented for one window, not multiple. Exiting early. TODO: support more windows. "
        );
        return;
    };
    let Some(size) = quad.computed_wh else {
        return;
    };

    for cursor_event in cursor_moved.read() {
        // A cursor outside the window still maps, because the ratio math is unbounded.
        let Ok(window) = windows.get(cursor_event.window) else {
            continue;
        };
        let window_size = Vec2::new(window.width(), window.height());
        let local = window_pointer_local(cursor_event.position, window_size, size);
        mouse_state.x = local.x;
        mouse_state.y = local.y;

        let pointer_event = BlitzPointerEvent {
            id: BlitzPointerId::Mouse,
            is_primary: true,
            coords: PointerCoords {
                page_x: local.x,
                page_y: local.y,
                screen_x: local.x,
                screen_y: local.y,
                client_x: local.x,
                client_y: local.y,
            },
            button: Default::default(),
            buttons: mouse_state.buttons,
            mods: mouse_state.mods,
            details: Default::default(),
            element: Default::default(),
            active_pointers: Default::default(),
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
    mouse_state: Res<MouseState>,
    routing: Res<MouseMessageRouting>,
    picking_state: Res<WorldSpacePickingState>,
) {
    // A latched gesture is mapped by the capture system instead.
    if mouse_state.gesture.is_some() {
        return;
    }

    if routing.window_space_handled {
        return;
    }

    if cursor_moved.is_empty() {
        return;
    }

    for _ in cursor_moved.read() {
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
                    element: Default::default(),
                    active_pointers: Default::default(),
                };
                let _ = worker
                    .input_tx
                    .try_send((pick.hit_entity, UiEvent::PointerMove(local_event)));
            }
        }
    }
}

/// Maps a bevy mouse button to the blitz equivalent.
fn blitz_button(button: MouseButton) -> Option<MouseEventButton> {
    match button {
        MouseButton::Left => Some(MouseEventButton::Main),
        MouseButton::Right => Some(MouseEventButton::Secondary),
        MouseButton::Middle => Some(MouseEventButton::Auxiliary),
        MouseButton::Back => Some(MouseEventButton::Fourth),
        MouseButton::Forward => Some(MouseEventButton::Fifth),
        _ => None,
    }
}

/// Resolves the document that owns a new press.
fn latch_gesture(
    mouse_state: &MouseState,
    pick_state: &DioxusUiPickState,
    picking_state: &WorldSpacePickingState,
    window_ui: &Query<Entity, With<DioxusWindowUiQuad>>,
) -> Option<GestureTarget> {
    match pick_state.active {
        DioxusUiPickFilter::WINDOW_SPACE => window_ui.single().ok().map(|entity| GestureTarget {
            entity,
            space: DioxusUiPickFilter::WINDOW_SPACE,
            local: Vec2::new(mouse_state.x, mouse_state.y),
            camera: None,
        }),
        DioxusUiPickFilter::WORLD_SPACE => picking_state.pick.as_ref().map(|pick| GestureTarget {
            entity: pick.hit_entity,
            space: DioxusUiPickFilter::WORLD_SPACE,
            local: pick.local_cords,
            camera: Some(pick.camera),
        }),
        _ => None,
    }
}

/// Builds a pointer event at a surface pixel position.
fn pointer_event_at(
    mouse_state: &MouseState,
    button: MouseEventButton,
    local: Vec2,
) -> BlitzPointerEvent {
    BlitzPointerEvent {
        id: BlitzPointerId::Mouse,
        is_primary: true,
        coords: PointerCoords {
            page_x: local.x,
            page_y: local.y,
            screen_x: local.x,
            screen_y: local.y,
            client_x: local.x,
            client_y: local.y,
        },
        button,
        buttons: mouse_state.buttons,
        mods: mouse_state.mods,
        details: Default::default(),
        element: Default::default(),
        active_pointers: Default::default(),
    }
}

/// Builds a pointer event for the document that owns the gesture.
fn build_pointer_event(
    mouse_state: &MouseState,
    button: MouseEventButton,
    target: GestureTarget,
) -> BlitzPointerEvent {
    let local = match target.space {
        DioxusUiPickFilter::WORLD_SPACE => target.local,
        _ => Vec2::new(mouse_state.x, mouse_state.y),
    };
    pointer_event_at(mouse_state, button, local)
}

/// Keeps a release inside the window so the document still receives it.
fn clamp_pointer_to_window(pointer: &mut BlitzPointerEvent, window: &Window) {
    let width = window.physical_width().max(1) as f32;
    let height = window.physical_height().max(1) as f32;
    let x = pointer.coords.page_x.clamp(0.0, width - 1.0);
    let y = pointer.coords.page_y.clamp(0.0, height - 1.0);
    pointer.coords = PointerCoords {
        page_x: x,
        page_y: y,
        screen_x: x,
        screen_y: y,
        client_x: x,
        client_y: y,
    };
}

/// Sends button press and release events to the document that owns the gesture.
pub(crate) fn blitz_mouse_button_handling(
    mut registry: NonSendMut<VdomThreadRegistry>,
    mut mouse_button_input_events: MessageReader<MouseButtonInput>,
    mut mouse_state: ResMut<MouseState>,
    pick_state: Res<DioxusUiPickState>,
    picking_state: Res<WorldSpacePickingState>,
    window_ui: Query<Entity, With<DioxusWindowUiQuad>>,
    windows: Query<&Window>,
    pointer_senders: Query<&DioxusPointerSender>,
) {
    for event in mouse_button_input_events.read() {
        let Some(button_blitz) = blitz_button(event.button) else {
            continue;
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

        // The first press picks its document from the current hit tests. Later
        // buttons and the matching releases reuse that document.
        let target = mouse_state.gesture.or_else(|| match event.state {
            ButtonState::Pressed => {
                latch_gesture(&mouse_state, &pick_state, &picking_state, &window_ui)
            }
            ButtonState::Released => None,
        });
        let Some(target) = target else {
            continue;
        };

        let mut pointer = build_pointer_event(&mouse_state, button_blitz, target);
        if matches!(event.state, ButtonState::Released)
            && target.space == DioxusUiPickFilter::WINDOW_SPACE
            && let Ok(window) = windows.single()
        {
            clamp_pointer_to_window(&mut pointer, window);
        }
        let ui_event = match event.state {
            ButtonState::Pressed => UiEvent::PointerDown(pointer),
            ButtonState::Released => UiEvent::PointerUp(pointer),
        };

        if let Some(worker) = registry.workers.get_mut(&target.entity) {
            let _ = worker.input_tx.try_send((target.entity, ui_event));
        }

        match event.state {
            ButtonState::Pressed => {
                mouse_state.gesture = Some(target);
                mouse_state.focused = Some(target.entity);
            }
            ButtonState::Released if mouse_state.buttons.is_empty() => {
                if let Ok(sender) = pointer_senders.get(target.entity) {
                    sender.send(CapturedPointer::Up);
                }
                mouse_state.gesture = None;
            }
            _ => {}
        }
    }
}

/// Maps the held pointer into the latched document and reports the move.
pub(crate) fn captured_pointer_moves(
    mut cursor_moved: MessageReader<CursorMoved>,
    mut mouse_state: ResMut<MouseState>,
    mut registry: NonSendMut<VdomThreadRegistry>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    world_quads: Query<(&DioxusUiQuad, &GlobalTransform), Without<DioxusWindowUiQuad>>,
    window_ui: Query<(Entity, &DioxusUiQuad), With<DioxusWindowUiQuad>>,
    windows: Query<&Window>,
    pointer_senders: Query<&DioxusPointerSender>,
) {
    let Some(mut gesture) = mouse_state.gesture else {
        return;
    };
    if mouse_state.buttons.is_empty() {
        return;
    }

    let mut moved = false;

    for event in cursor_moved.read() {
        let Ok(window) = windows.get(event.window) else {
            continue;
        };

        let local = match gesture.space {
            DioxusUiPickFilter::WINDOW_SPACE => {
                let Ok((_, quad)) = window_ui.single() else {
                    continue;
                };
                let Some(size) = quad.computed_wh else {
                    continue;
                };
                // Cursor positions outside the window still map by ratio.
                let window_size = Vec2::new(window.width(), window.height());
                let local = window_pointer_local(event.position, window_size, size);
                mouse_state.x = local.x;
                mouse_state.y = local.y;
                Some(local)
            }
            DioxusUiPickFilter::WORLD_SPACE => {
                let Some(camera_entity) = gesture.camera else {
                    continue;
                };
                let Ok((camera, camera_transform)) = cameras.get(camera_entity) else {
                    continue;
                };
                let Ok((quad, quad_transform)) = world_quads.get(gesture.entity) else {
                    continue;
                };
                let physical = event.position * window.scale_factor();
                world_pointer_local(physical, camera, camera_transform, quad, quad_transform)
            }
            _ => None,
        };

        // A degenerate ray keeps the last good position.
        let Some(local) = local else {
            continue;
        };
        gesture.local = local;
        moved = true;
    }

    if !moved {
        return;
    }

    mouse_state.gesture = Some(gesture);

    let pointer_event = pointer_event_at(&mouse_state, Default::default(), gesture.local);
    if let Some(worker) = registry.workers.get_mut(&gesture.entity) {
        let _ = worker
            .input_tx
            .try_send((gesture.entity, UiEvent::PointerMove(pointer_event)));
    }
    if let Ok(sender) = pointer_senders.get(gesture.entity) {
        let pointer = Vec2Px {
            x: gesture.local.x.round() as i32,
            y: gesture.local.y.round() as i32,
        };
        sender.send(CapturedPointer::Move(pointer));
    }
}

/// Sends wheel events to the document under the pointer.
pub(crate) fn blitz_mouse_wheel_handling(
    mut registry: NonSendMut<VdomThreadRegistry>,
    mut mouse_wheel_events: MessageReader<MouseWheel>,
    mouse_state: Res<MouseState>,
    pick_state: Res<DioxusUiPickState>,
    picking_state: Res<WorldSpacePickingState>,
    window_ui: Query<Entity, With<DioxusWindowUiQuad>>,
) {
    for event in mouse_wheel_events.read() {
        let Some(target) = latch_gesture(&mouse_state, &pick_state, &picking_state, &window_ui)
        else {
            continue;
        };

        let (x, y) = match target.space {
            DioxusUiPickFilter::WORLD_SPACE => (target.local.x, target.local.y),
            _ => (mouse_state.x, mouse_state.y),
        };

        let delta = match event.unit {
            MouseScrollUnit::Line => BlitzWheelDelta::Lines(event.x as f64, event.y as f64),
            MouseScrollUnit::Pixel => BlitzWheelDelta::Pixels(event.x as f64, event.y as f64),
        };

        let wheel_event = BlitzWheelEvent {
            delta,
            coords: PointerCoords {
                page_x: x,
                page_y: y,
                screen_x: x,
                screen_y: y,
                client_x: x,
                client_y: y,
            },
            buttons: mouse_state.buttons,
            mods: mouse_state.mods,
            element: Default::default(),
        };

        if let Some(worker) = registry.workers.get_mut(&target.entity) {
            let _ = worker
                .input_tx
                .try_send((target.entity, UiEvent::Wheel(wheel_event)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::{App, PreUpdate};
    use bevy_camera::{
        CameraProjection, OrthographicProjection, RenderTargetInfo, ScalingMode,
        Viewport as CameraViewport,
    };
    use bevy_dioxus_render::worker::{VdomCommand, VdomResult, VdomWorker};
    use bevy_ecs::schedule::IntoScheduleConfigs;
    use bevy_transform::components::Transform;
    use crossbeam_channel::{Receiver, Sender};
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    /// Builds an orthographic camera that sees a two by one area.
    fn test_camera(physical_size: Vec2) -> Camera {
        let viewport = CameraViewport {
            physical_size: physical_size.as_uvec2(),
            ..Default::default()
        };
        let mut camera = Camera {
            viewport: Some(viewport.clone()),
            ..Default::default()
        };
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: viewport.physical_size,
            scale_factor: 1.0,
        });
        let mut projection = OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: 2.0,
                height: 1.0,
            },
            ..OrthographicProjection::default_3d()
        };
        projection.update(physical_size.x, physical_size.y);
        camera.computed.clip_from_view = projection.get_clip_from_view();
        camera
    }

    /// Builds a quad with a two by one local area and a hundred by fifty pixel surface.
    fn test_quad() -> DioxusUiQuad {
        DioxusUiQuad {
            handle: None,
            computed_wh: Some(Vec2::new(100.0, 50.0)),
            local_half_extents: Some(Vec2::new(1.0, 0.5)),
        }
    }

    struct Harness {
        app: App,
        window_ui: Entity,
        world_ui: Entity,
        window_rx: Receiver<(Entity, UiEvent)>,
        world_rx: Receiver<(Entity, UiEvent)>,
        _keepalive: (Vec<Receiver<VdomCommand>>, Vec<Sender<VdomResult>>),
    }

    /// Builds a worker with no thread so the input channel can be inspected.
    fn idle_worker() -> (
        VdomWorker,
        Receiver<(Entity, UiEvent)>,
        Receiver<VdomCommand>,
        Sender<VdomResult>,
    ) {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let (result_tx, result_rx) = crossbeam_channel::unbounded();
        let (input_tx, input_rx) = crossbeam_channel::unbounded();
        let worker = VdomWorker {
            cmd_tx,
            result_rx,
            input_tx,
            waker_flag: Arc::new(AtomicBool::new(false)),
            thread: None,
        };
        (worker, input_rx, cmd_rx, result_tx)
    }

    impl Harness {
        fn new() -> Self {
            let mut app = App::new();
            app.add_message::<MouseButtonInput>()
                .init_resource::<MouseState>()
                .init_resource::<DioxusUiPickState>()
                .init_resource::<WorldSpacePickingState>()
                .add_systems(PreUpdate, blitz_mouse_button_handling);

            let window_ui = app.world_mut().spawn(DioxusWindowUiQuad).id();
            let world_ui = app.world_mut().spawn_empty().id();
            app.world_mut().spawn(Window::default());

            let (window_worker, window_rx, window_cmd, window_result) = idle_worker();
            let (world_worker, world_rx, world_cmd, world_result) = idle_worker();

            let mut registry = VdomThreadRegistry::default();
            registry.workers.insert(window_ui, window_worker);
            registry.workers.insert(world_ui, world_worker);
            app.world_mut().insert_non_send(registry);

            Self {
                app,
                window_ui,
                world_ui,
                window_rx,
                world_rx,
                _keepalive: (
                    vec![window_cmd, world_cmd],
                    vec![window_result, world_result],
                ),
            }
        }

        fn send_left_button(&mut self, state: ButtonState) {
            let window = self.window_ui;
            let _ = self.app.world_mut().write_message(MouseButtonInput {
                button: MouseButton::Left,
                state,
                window,
            });
        }

        fn route_to_window_space(&mut self) {
            self.app
                .world_mut()
                .resource_mut::<DioxusUiPickState>()
                .active = DioxusUiPickFilter::WINDOW_SPACE;
        }

        fn route_to_world_space(&mut self) {
            let world_ui = self.world_ui;
            self.app
                .world_mut()
                .resource_mut::<DioxusUiPickState>()
                .active = DioxusUiPickFilter::WORLD_SPACE;
            self.app
                .world_mut()
                .resource_mut::<WorldSpacePickingState>()
                .pick = Some(UiPickState {
                hit_entity: world_ui,
                local_cords: Vec2::new(12.0, 34.0),
                world_cords: Vec3::ZERO,
                camera: Entity::PLACEHOLDER,
            });
        }

        fn drain(rx: &Receiver<(Entity, UiEvent)>) -> Vec<(Entity, UiEvent)> {
            let mut out = Vec::new();
            while let Ok(item) = rx.try_recv() {
                out.push(item);
            }
            out
        }
    }

    fn pointer_downs(events: &[(Entity, UiEvent)]) -> usize {
        events
            .iter()
            .filter(|(_, event)| matches!(event, UiEvent::PointerDown(_)))
            .count()
    }

    #[test]
    fn release_outside_the_window_still_targets_the_document() {
        let mut harness = Harness::new();
        harness.route_to_window_space();
        {
            let mut state = harness.app.world_mut().resource_mut::<MouseState>();
            state.x = 4_000.0;
            state.y = 4_000.0;
        }
        harness.send_left_button(ButtonState::Pressed);
        harness.app.update();
        let _ = Harness::drain(&harness.window_rx);

        harness.send_left_button(ButtonState::Released);
        harness.app.update();

        let events = Harness::drain(&harness.window_rx);
        let Some((_, UiEvent::PointerUp(event))) = events.last() else {
            panic!("expected a pointer up");
        };
        assert!(event.coords.client_x < 1280.0);
        assert!(event.coords.client_y < 720.0);
    }

    #[test]
    fn press_forwards_one_pointer_down_across_many_ticks() {
        let mut harness = Harness::new();
        harness.route_to_world_space();
        harness.send_left_button(ButtonState::Pressed);

        for _ in 0..4 {
            harness.app.update();
        }

        assert_eq!(pointer_downs(&Harness::drain(&harness.world_rx)), 1);
        assert!(Harness::drain(&harness.window_rx).is_empty());
    }

    #[test]
    fn resident_press_is_not_forwarded_without_a_buffer_rotation() {
        let mut harness = Harness::new();
        harness.route_to_world_space();
        harness.send_left_button(ButtonState::Pressed);

        for _ in 0..3 {
            harness.app.world_mut().run_schedule(PreUpdate);
        }

        assert_eq!(pointer_downs(&Harness::drain(&harness.world_rx)), 1);
    }

    #[test]
    fn rapid_presses_keep_their_order() {
        let mut harness = Harness::new();
        harness.route_to_world_space();
        for _ in 0..4 {
            harness.send_left_button(ButtonState::Pressed);
            harness.send_left_button(ButtonState::Released);
        }

        harness.app.update();

        let kinds: Vec<&str> = Harness::drain(&harness.world_rx)
            .iter()
            .map(|(_, event)| match event {
                UiEvent::PointerDown(_) => "down",
                UiEvent::PointerUp(_) => "up",
                _ => "other",
            })
            .collect();
        assert_eq!(
            kinds,
            vec!["down", "up", "down", "up", "down", "up", "down", "up"]
        );
    }

    #[test]
    fn release_stays_with_the_document_that_received_the_press() {
        let mut harness = Harness::new();
        harness.route_to_world_space();
        harness.send_left_button(ButtonState::Pressed);
        harness.app.update();
        assert_eq!(pointer_downs(&Harness::drain(&harness.world_rx)), 1);

        // Routing moves to window space before the release arrives.
        harness
            .app
            .world_mut()
            .resource_mut::<DioxusUiPickState>()
            .active = DioxusUiPickFilter::WINDOW_SPACE;
        harness
            .app
            .world_mut()
            .resource_mut::<WorldSpacePickingState>()
            .pick = None;

        harness.send_left_button(ButtonState::Released);
        harness.app.update();

        let world_events = Harness::drain(&harness.world_rx);
        assert_eq!(world_events.len(), 1);
        assert!(matches!(&world_events[0].1, UiEvent::PointerUp(_)));
        assert_eq!(world_events[0].0, harness.world_ui);
        assert!(Harness::drain(&harness.window_rx).is_empty());
    }

    #[test]
    fn press_over_open_space_delivers_nothing() {
        let mut harness = Harness::new();
        harness
            .app
            .world_mut()
            .resource_mut::<DioxusUiPickState>()
            .active = DioxusUiPickFilter::NONE;

        harness.send_left_button(ButtonState::Pressed);
        harness.app.update();
        harness.send_left_button(ButtonState::Released);
        harness.app.update();

        assert!(Harness::drain(&harness.world_rx).is_empty());
        assert!(Harness::drain(&harness.window_rx).is_empty());
    }

    #[test]
    fn overlay_camera_hits_do_not_clear_the_world_pick() {
        use bevy_picking::backend::{HitData, PointerHits};
        use bevy_picking::pointer::PointerId;

        let mut app = App::new();
        app.add_message::<PointerHits>()
            .init_resource::<WorldSpacePickingState>()
            .add_systems(PreUpdate, update_world_space_picking);

        let quad = app
            .world_mut()
            .spawn((
                DioxusUiQuad {
                    handle: None,
                    computed_wh: Some(Vec2::new(100.0, 50.0)),
                    local_half_extents: Some(Vec2::new(1.0, 0.5)),
                },
                GlobalTransform::default(),
            ))
            .id();
        let overlay = app.world_mut().spawn_empty().id();
        let world_camera = app.world_mut().spawn_empty().id();
        let overlay_camera = app.world_mut().spawn_empty().id();

        let hits = |entity, camera| {
            PointerHits::new(
                PointerId::Mouse,
                vec![(
                    entity,
                    HitData::new(camera, 1.0, Some(Vec3::ZERO), Some(Vec3::Z)),
                )],
                0.0,
            )
        };

        let _ = app.world_mut().write_message(hits(quad, world_camera));
        let _ = app.world_mut().write_message(hits(overlay, overlay_camera));
        app.update();

        let picked = app
            .world()
            .resource::<WorldSpacePickingState>()
            .pick
            .as_ref()
            .map(|pick| pick.hit_entity);
        assert_eq!(picked, Some(quad));
    }

    #[test]
    fn pick_clears_when_a_frame_reports_no_world_quad() {
        use bevy_picking::backend::{HitData, PointerHits};
        use bevy_picking::pointer::PointerId;

        let mut app = App::new();
        app.add_message::<PointerHits>()
            .init_resource::<WorldSpacePickingState>()
            .add_systems(PreUpdate, update_world_space_picking);

        let quad = app
            .world_mut()
            .spawn((
                DioxusUiQuad {
                    handle: None,
                    computed_wh: Some(Vec2::new(100.0, 50.0)),
                    local_half_extents: Some(Vec2::new(1.0, 0.5)),
                },
                GlobalTransform::default(),
            ))
            .id();
        let overlay = app.world_mut().spawn_empty().id();
        let camera = app.world_mut().spawn_empty().id();

        let hits = |entity| {
            PointerHits::new(
                PointerId::Mouse,
                vec![(
                    entity,
                    HitData::new(camera, 1.0, Some(Vec3::ZERO), Some(Vec3::Z)),
                )],
                0.0,
            )
        };

        let _ = app.world_mut().write_message(hits(quad));
        app.update();
        assert!(
            app.world()
                .resource::<WorldSpacePickingState>()
                .pick
                .is_some()
        );

        // The pointer left the quad, so the overlay camera is all that reports.
        let _ = app.world_mut().write_message(hits(overlay));
        app.update();
        assert!(
            app.world()
                .resource::<WorldSpacePickingState>()
                .pick
                .is_none()
        );
    }

    #[test]
    fn world_space_requires_a_live_pick() {
        let mut app = App::new();
        app.init_resource::<DioxusUiHitState>()
            .init_resource::<WorldSpacePickingState>()
            .init_resource::<DioxusUiPickState>()
            .init_resource::<MouseState>()
            .add_systems(PreUpdate, resolve_pick_space);

        app.world_mut()
            .resource_mut::<DioxusUiHitState>()
            .world_space_caught = true;
        app.update();
        assert_eq!(
            app.world().resource::<DioxusUiPickState>().active,
            DioxusUiPickFilter::NONE
        );

        let target = app.world_mut().spawn_empty().id();
        app.world_mut()
            .resource_mut::<WorldSpacePickingState>()
            .pick = Some(UiPickState {
            hit_entity: target,
            local_cords: Vec2::ZERO,
            world_cords: Vec3::ZERO,
            camera: Entity::PLACEHOLDER,
        });
        app.update();
        assert_eq!(
            app.world().resource::<DioxusUiPickState>().active,
            DioxusUiPickFilter::WORLD_SPACE
        );
    }

    #[test]
    fn window_pointer_local_maps_outside_the_window() {
        let window = Vec2::new(800.0, 600.0);
        let quad = Vec2::new(1600.0, 1200.0);
        assert_eq!(
            window_pointer_local(Vec2::new(400.0, 300.0), window, quad),
            Vec2::new(800.0, 600.0)
        );
        assert_eq!(
            window_pointer_local(Vec2::new(-100.0, -50.0), window, quad),
            Vec2::new(-200.0, -100.0)
        );
        assert_eq!(
            window_pointer_local(Vec2::new(1200.0, 900.0), window, quad),
            Vec2::new(2400.0, 1800.0)
        );
    }

    #[test]
    fn world_pointer_local_maps_points_outside_the_quad() {
        let camera = test_camera(Vec2::new(800.0, 600.0));
        let camera_transform = GlobalTransform::default();
        let quad = test_quad();
        let quad_transform = GlobalTransform::from_xyz(0.0, 0.0, -5.0);

        let local = |cursor: Vec2| {
            world_pointer_local(cursor, &camera, &camera_transform, &quad, &quad_transform)
                .expect("the quad plane is in front of the camera")
        };

        assert!(local(Vec2::new(400.0, 300.0)).abs_diff_eq(Vec2::new(50.0, 25.0), 1e-3));
        assert!(local(Vec2::new(1200.0, 300.0)).abs_diff_eq(Vec2::new(150.0, 25.0), 1e-3));
        assert!(local(Vec2::new(-400.0, 300.0)).abs_diff_eq(Vec2::new(-50.0, 25.0), 1e-3));
        assert!(local(Vec2::new(400.0, -300.0)).abs_diff_eq(Vec2::new(50.0, -25.0), 1e-3));
    }

    #[test]
    fn world_pointer_local_bounds_grazing_angles() {
        let camera = test_camera(Vec2::new(800.0, 600.0));
        let camera_transform = GlobalTransform::default();
        let quad = test_quad();
        let quad_transform = GlobalTransform::from_xyz(0.0, 0.0, -5.0);

        let local = world_pointer_local(
            Vec2::new(1_000_000.0, 300.0),
            &camera,
            &camera_transform,
            &quad,
            &quad_transform,
        )
        .expect("the quad plane is in front of the camera");

        assert!(local.is_finite());
        assert!(local.x <= 250.0 + 1e-3);
    }

    #[test]
    fn world_pointer_local_rejects_parallel_and_behind_rays() {
        let camera = test_camera(Vec2::new(800.0, 600.0));
        let camera_transform = GlobalTransform::default();
        let quad = test_quad();
        let cursor = Vec2::new(400.0, 300.0);

        let parallel = GlobalTransform::from(Transform {
            translation: Vec3::new(0.0, 0.0, -5.0),
            rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            ..Transform::default()
        });
        assert!(
            world_pointer_local(cursor, &camera, &camera_transform, &quad, &parallel).is_none()
        );

        let behind = GlobalTransform::from_xyz(0.0, 0.0, 5.0);
        assert!(world_pointer_local(cursor, &camera, &camera_transform, &quad, &behind).is_none());
    }

    #[test]
    fn captured_move_outside_the_quad_keeps_the_gesture_and_routes_only_to_it() {
        let mut app = App::new();
        app.add_message::<CursorMoved>()
            .add_message::<MouseButtonInput>()
            .init_resource::<MouseState>()
            .init_resource::<DioxusUiPickState>()
            .init_resource::<WorldSpacePickingState>()
            .add_systems(
                PreUpdate,
                (captured_pointer_moves, blitz_mouse_button_handling).chain(),
            );

        let camera = app
            .world_mut()
            .spawn((
                test_camera(Vec2::new(800.0, 600.0)),
                GlobalTransform::default(),
            ))
            .id();
        let world_ui = app
            .world_mut()
            .spawn((test_quad(), GlobalTransform::from_xyz(0.0, 0.0, -5.0)))
            .id();
        let window_ui = app.world_mut().spawn(DioxusWindowUiQuad).id();
        let window = app.world_mut().spawn(Window::default()).id();

        let (world_worker, world_rx, world_cmd, world_result) = idle_worker();
        let (window_worker, window_rx, window_cmd, window_result) = idle_worker();
        let mut registry = VdomThreadRegistry::default();
        registry.workers.insert(world_ui, world_worker);
        registry.workers.insert(window_ui, window_worker);
        app.world_mut().insert_non_send(registry);

        let (world_pointer_tx, world_pointer_rx) = crossbeam_channel::unbounded();
        let (window_pointer_tx, window_pointer_rx) = crossbeam_channel::unbounded();
        app.world_mut()
            .entity_mut(world_ui)
            .insert(DioxusPointerSender::new(world_pointer_tx));
        app.world_mut()
            .entity_mut(window_ui)
            .insert(DioxusPointerSender::new(window_pointer_tx));

        {
            let mut state = app.world_mut().resource_mut::<MouseState>();
            state.buttons = MouseEventButtons::from(MouseEventButton::Main);
            state.gesture = Some(GestureTarget {
                entity: world_ui,
                space: DioxusUiPickFilter::WORLD_SPACE,
                local: Vec2::new(50.0, 25.0),
                camera: Some(camera),
            });
        }

        let _ = app.world_mut().write_message(CursorMoved {
            window,
            position: Vec2::new(1200.0, 300.0),
            delta: Some(Vec2::ZERO),
        });
        app.update();

        let gesture = app
            .world()
            .resource::<MouseState>()
            .gesture
            .expect("the gesture must stay latched outside the quad");
        assert!(gesture.local.abs_diff_eq(Vec2::new(150.0, 25.0), 1e-3));

        let world_events = Harness::drain(&world_rx);
        assert!(matches!(
            world_events.last().map(|(_, event)| event),
            Some(UiEvent::PointerMove(_))
        ));
        assert!(Harness::drain(&window_rx).is_empty());
        assert_eq!(
            world_pointer_rx.try_recv(),
            Ok(CapturedPointer::Move(Vec2Px { x: 150, y: 25 }))
        );
        assert!(window_pointer_rx.try_recv().is_err());

        let _ = app.world_mut().write_message(CursorMoved {
            window,
            position: Vec2::new(800.0, 300.0),
            delta: Some(Vec2::ZERO),
        });
        let _ = app.world_mut().write_message(MouseButtonInput {
            button: MouseButton::Left,
            state: ButtonState::Released,
            window,
        });
        app.update();

        assert!(app.world().resource::<MouseState>().gesture.is_none());
        let world_events = Harness::drain(&world_rx);
        let Some((_, UiEvent::PointerUp(event))) = world_events.last() else {
            panic!("expected a pointer up on the latched document");
        };
        assert_eq!(event.coords.client_x, 100.0);
        assert_eq!(event.coords.client_y, 25.0);
        let captured: Vec<CapturedPointer> = world_pointer_rx.try_iter().collect();
        assert_eq!(
            captured,
            vec![
                CapturedPointer::Move(Vec2Px { x: 100, y: 25 }),
                CapturedPointer::Up,
            ]
        );
        assert!(Harness::drain(&window_rx).is_empty());

        let _keepalive = (
            vec![world_cmd, window_cmd],
            vec![world_result, window_result],
        );
    }
}
