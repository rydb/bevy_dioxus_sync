use bevy_app::Plugin;
use bevy_app::PreUpdate;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_input::InputSystem;

use super::{
    keyboard::handle_keyboard_events, mouse::handle_mouse_events, window::handle_window_resize,
};

pub struct DioxusEventSyncPlugin;

impl Plugin for DioxusEventSyncPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (
                handle_window_resize,
                handle_mouse_events.after(InputSystem),
                handle_keyboard_events.after(InputSystem),
            )
                .chain(),
        );
    }
}
