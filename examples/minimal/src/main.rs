//! minimal example showing each of the hooks

use bevy::prelude::*;
use bevy_dioxus_sync::plugins::DioxusPlugin;
use frontend::ui::app_ui;

use crate::bevy_scene_plugin::BevyScenePlugin;

mod bevy_scene_plugin;

pub fn main() {
    println!("running bevy plugin");
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DioxusPlugin {
            bevy_info_refresh_fps: 30,
            main_window_ui: Some(app_ui),
        })
        .add_plugins(BevyScenePlugin)
        .run();
}
