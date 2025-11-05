use bevy_app::Plugin;
use bevy_app::PreUpdate;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_input::InputSystems;

use super::{
    keyboard::handle_keyboard_messages, mouse::handle_mouse_messages, window::handle_window_resize,
};

pub struct DioxusEventSyncPlugin;

impl Plugin for DioxusEventSyncPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (
                handle_window_resize,
                handle_mouse_messages.after(InputSystems),
                handle_keyboard_messages.after(InputSystems),
            )
                .chain(),
        );
    }
}
