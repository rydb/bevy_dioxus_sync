//! minimal example showing each of the hooks

use bevy::prelude::*;
use bevy_dioxus_sync::plugins::DioxusPlugin;

use crate::{backend::bevy_scene_plugin::BevyScenePlugin, frontend::app_ui};

pub mod backend;
pub mod frontend;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DioxusPlugin {
            bevy_info_refresh_fps: 30,
            main_window_ui: Some(app_ui),
        })
        .add_plugins(BevyScenePlugin)
        .run();
}
