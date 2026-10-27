use bevy_app::Plugin;
use bevy_app::PreUpdate;
use bevy_dioxus_render::{
    DioxusInputRouting, DioxusUiHitState, DioxusUiPickState, drain_worker_results,
};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_input::InputSystems;
use bevy_picking::PickingSystems;

use crate::mouse::MouseMessageRouting;
use crate::mouse::MouseState;
use crate::mouse::WorldSpacePickingState;

use super::{
    keyboard::handle_keyboard_messages, mouse::blitz_mouse_button_handling,
    mouse::resolve_pick_space, mouse::update_world_space_picking,
    mouse::window_space_mouse_messages, mouse::world_space_mouse_messages,
};

pub struct DioxusEventSyncPlugin;

impl Plugin for DioxusEventSyncPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<WorldSpacePickingState>()
            .init_resource::<DioxusUiPickState>()
            .init_resource::<DioxusUiHitState>()
            .init_resource::<MouseState>()
            .init_resource::<MouseMessageRouting>()
            .add_systems(
                PreUpdate,
                (
                    update_world_space_picking.after(PickingSystems::Backend),
                    resolve_pick_space
                        .after(update_world_space_picking)
                        .after(drain_worker_results)
                        .in_set(DioxusInputRouting),
                    window_space_mouse_messages
                        .after(InputSystems)
                        .after(DioxusInputRouting),
                    world_space_mouse_messages.after(window_space_mouse_messages),
                    blitz_mouse_button_handling.after(world_space_mouse_messages),
                    handle_keyboard_messages.after(blitz_mouse_button_handling),
                ),
            );
    }
}
