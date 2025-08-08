use std::sync::atomic::AtomicPtr;

use bevy::gizmos::cross;
use bevy::prelude::*;
use dioxus_bevy_panel::dioxus_in_bevy_plugin::DioxusInBevyPlugin;

use crate::bevy_scene_plugin::CubeRotationSpeed;
use crate::{bevy_scene_plugin::BevyScenePlugin};

mod bevy_scene_plugin;
mod ui;

fn main() {
    // ui_messages.register::<UIMessage>();
    // let props = DioxusProps {
    //     ui_messages
    // };
    App::new()
        .add_plugins(DefaultPlugins)
        // .add_plugins()
        .add_plugins(DioxusInBevyPlugin {})
        .add_plugins(BevyScenePlugin)
        .run();
}
