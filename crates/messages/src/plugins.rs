use bevy_app::Plugin;
use bevy_app::PreUpdate;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_input::InputSystems;
use bevy_picking::PickingSystems;

use super::{
    keyboard::handle_keyboard_messages, mouse::handle_mouse_messages,
    mouse::update_world_space_picking,
};

pub struct DioxusEventSyncPlugin;

impl Plugin for DioxusEventSyncPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<super::mouse::WorldSpacePickingState>()
            .add_systems(
                PreUpdate,
                (
                    update_world_space_picking.after(PickingSystems::Backend),
                    handle_mouse_messages.after(InputSystems),
                    handle_keyboard_messages.after(InputSystems),
                )
                    .chain(),
            );
    }
}
