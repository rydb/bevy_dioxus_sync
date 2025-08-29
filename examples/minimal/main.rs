//! minimal example showing each of the hooks

use bevy::prelude::*;
use bevy_dioxus_sync::dioxus_in_bevy_plugin::DioxusPlugin;

use crate::bevy_scene_plugin::BevyScenePlugin;

mod bevy_scene_plugin;
mod ui;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DioxusPlugin {})
        .add_plugins(BevyScenePlugin)
        .run();
}
