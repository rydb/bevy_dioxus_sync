//! minimal example showing each of the hooks

use bevy::prelude::*;
use bevy_dioxus_sync::{plugins::{DioxusAppKind, DioxusPlugin, DioxusPropsNative}, ui::dioxus_app};
use dioxus::core::VirtualDom;
// use dioxus_desktop::Config;
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
    // let vdom = VirtualDom::new_with_props(dioxus_app, DioxusAppKind::NativeOnly(DioxusPropsNative {
    //     fps: 30,
    //     main_window_ui: Some(app_ui),
    // }));
    // // let builder = dioxus::LaunchBuilder::new().with_context_provider(state);
    // dioxus_desktop::launch::launch_virtual_dom(vdom, Config::new())
    // // dioxus::launch(vdom);
    // // let builder = dioxus::LaunchBuilder {
    // //     platform: KnownPlatform::native
    // }
}

fn bevy_main() {
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
