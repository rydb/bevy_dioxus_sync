use bevy_dioxus_render::worker::VdomThreadRegistry;
use bevy_dioxus_render::{
    DioxusUiHitState, DioxusUiPickFilter, DioxusUiPickState, DioxusUiQuad, DioxusWindowUiQuad,
};
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
    /// Pointer position inside the document at press time.
    pub local: Vec2,
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

/// Resolves the owning pick space from the reported hit tests and the live pick.
pub(crate) fn resolve_pick_space(
    hits: Res<DioxusUiHitState>,
    picking_state: Res<WorldSpacePickingState>,
    mut pick_state: ResMut<DioxusUiPickState>,
) {
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

            let u = (local_pos.x + half.x) / (2.0 * half.x);
            let v = (local_pos.y + half.y) / (2.0 * half.y);

            let pixel_x = u * wh.x;
            let pixel_y = (1.0 - v) * wh.y;

            pick = Some(UiPickState {
                hit_entity: *entity,
                local_cords: Vec2::new(pixel_x, pixel_y),
                world_cords: world_pos,
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
    window_ui: Query<Entity, With<DioxusWindowUiQuad>>,
) {
    routing.window_space_handled = pick_state.active.contains(DioxusUiPickFilter::WINDOW_SPACE);

    if cursor_moved.is_empty() {
        return;
    }

    let Ok(window_ui) = window_ui.single() else {
        error!(
            "This system is only implemented for one window, not multiple. Exiting early. TODO: support more windows. "
        );
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
    pick_state: &DioxusUiPickState,
    picking_state: &WorldSpacePickingState,
    window_ui: &Query<Entity, With<DioxusWindowUiQuad>>,
) -> Option<GestureTarget> {
    match pick_state.active {
        DioxusUiPickFilter::WINDOW_SPACE => window_ui.single().ok().map(|entity| GestureTarget {
            entity,
            space: DioxusUiPickFilter::WINDOW_SPACE,
            local: Vec2::ZERO,
        }),
        DioxusUiPickFilter::WORLD_SPACE => picking_state.pick.as_ref().map(|pick| GestureTarget {
            entity: pick.hit_entity,
            space: DioxusUiPickFilter::WORLD_SPACE,
            local: pick.local_cords,
        }),
        _ => None,
    }
}

/// Builds a pointer event for the document that owns the gesture.
fn build_pointer_event(
    mouse_state: &MouseState,
    button: MouseEventButton,
    target: GestureTarget,
) -> BlitzPointerEvent {
    let (x, y) = match target.space {
        DioxusUiPickFilter::WORLD_SPACE => (target.local.x, target.local.y),
        _ => (mouse_state.x, mouse_state.y),
    };

    BlitzPointerEvent {
        id: BlitzPointerId::Mouse,
        is_primary: true,
        coords: PointerCoords {
            page_x: x,
            page_y: y,
            screen_x: x,
            screen_y: y,
            client_x: x,
            client_y: y,
        },
        button,
        buttons: mouse_state.buttons,
        mods: mouse_state.mods,
        details: Default::default(),
        element: Default::default(),
        active_pointers: Default::default(),
    }
}

/// Sends button press and release events to the document that owns the gesture.
pub(crate) fn blitz_mouse_button_handling(
    mut registry: NonSendMut<VdomThreadRegistry>,
    mut mouse_button_input_events: MessageReader<MouseButtonInput>,
    mut mouse_state: ResMut<MouseState>,
    pick_state: Res<DioxusUiPickState>,
    picking_state: Res<WorldSpacePickingState>,
    window_ui: Query<Entity, With<DioxusWindowUiQuad>>,
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
            ButtonState::Pressed => latch_gesture(&pick_state, &picking_state, &window_ui),
            ButtonState::Released => None,
        });
        let Some(target) = target else {
            continue;
        };

        let pointer = build_pointer_event(&mouse_state, button_blitz, target);
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
                mouse_state.gesture = None;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::{App, PreUpdate};
    use bevy_dioxus_render::worker::{VdomCommand, VdomResult, VdomWorker};
    use crossbeam_channel::{Receiver, Sender};
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

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
        });
        app.update();
        assert_eq!(
            app.world().resource::<DioxusUiPickState>().active,
            DioxusUiPickFilter::WORLD_SPACE
        );
    }
}
