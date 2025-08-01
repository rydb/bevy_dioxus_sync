use bevy::gizmos::cross;
use bevy::prelude::*;
use dioxus_bevy_panel::dioxus_in_bevy_plugin::DioxusInBevyPlugin;
use dioxus_bevy_panel::{UiMessageRegistration, UiMessageRegistry};

use crate::bevy_scene_plugin::CubeRotationSpeed;
use crate::ui::{dioxus_app, UIMessage};
use crate::{bevy_scene_plugin::BevyScenePlugin};

mod bevy_scene_plugin;
mod ui;

fn main() {
    // ui_messages.register::<UIMessage>();
    // let props = UIProps {
    //     ui_messages
    // };

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DioxusInBevyPlugin { ui: dioxus_app })
        .add_plugins(BevyScenePlugin)
        .run();
}
