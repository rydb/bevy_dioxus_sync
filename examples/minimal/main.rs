use bevy::prelude::*;
use dioxus_bevy_panel::dioxus_in_bevy_plugin::DioxusInBevyPlugin;

use crate::{bevy_scene_plugin::BevyScenePlugin, ui::UIProps};
use crate::ui::ui;

mod bevy_scene_plugin;
mod ui;

fn main() {
    let (ui_sender, ui_receiver) = crossbeam_channel::unbounded();
    let (app_sender, app_receiver) = crossbeam_channel::unbounded();
    let props = UIProps {
        ui_sender,
        app_receiver,
    };

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DioxusInBevyPlugin::<UIProps> { ui, props })
        .add_plugins(BevyScenePlugin {
            app_sender,
            ui_receiver,
        })
        .run();
}
