//! minimal example showing each of the hooks

use bevy::prelude::*;
use bevy_dioxus_sync::plugins::DioxusPlugin;
use frontend::ui::app_ui;

use crate::bevy_scene_plugin::BevyScenePlugin;


mod bevy_scene_plugin;

pub enum ENVChoice {
    Dioxus,
    Bevy,
}

const CHOICE: ENVChoice = ENVChoice::Bevy;

pub fn main() {
    match CHOICE {
        ENVChoice::Dioxus => ui_debug_main(),
        ENVChoice::Bevy => bevy_main(),
    }
}

fn ui_debug_main() {
    dioxus::launch(app_ui);
}

fn bevy_main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DioxusPlugin {
            bevy_info_refresh_fps: 30,
            main_window_ui: Some(app_ui),
        })
        .add_plugins(BevyScenePlugin)
        .run();
    
}
