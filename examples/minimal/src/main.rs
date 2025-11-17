//! minimal example showing each of the hooks

use bevy::prelude::*;
use bevy_dioxus_sync::{panels::DioxusPanel, plugins::DioxusPlugin};

use crate::{backend::bevy_scene_plugin::BevyScenePlugin, frontend::AppUi};

pub mod backend;
pub mod frontend;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DioxusPlugin {
            bevy_info_refresh_fps: 30,
            main_window_ui: Some(DioxusPanel::new(AppUi {})),
        })
        .add_plugins(BevyScenePlugin)
        .run();
}
