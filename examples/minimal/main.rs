use std::sync::atomic::AtomicPtr;

use bevy::gizmos::cross;
use bevy::prelude::*;
use dioxus::signals::SyncSignal;
use dioxus_bevy_panel::dioxus_in_bevy_plugin::DioxusPlugin;

use crate::bevy_scene_plugin::CubeRotationSpeed;
use crate::{bevy_scene_plugin::BevyScenePlugin};

mod bevy_scene_plugin;
mod ui;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // .add_plugins()
        .add_plugins(DioxusPlugin {})
        .add_plugins(BevyScenePlugin)
        .run();
}
