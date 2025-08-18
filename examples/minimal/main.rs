use std::sync::atomic::AtomicPtr;

use bevy::gizmos::cross;
use bevy::prelude::*;
use dioxus::signals::SyncSignal;
use dioxus_bevy_panel::dioxus_in_bevy_plugin::DioxusInBevyPlugin;

use crate::bevy_scene_plugin::CubeRotationSpeed;
use crate::{bevy_scene_plugin::BevyScenePlugin};

mod bevy_scene_plugin;
mod ui;


pub enum BevyQueryScope {
    Entity(Entity),
    World
}

pub struct BevyQuery<T: Component + Clone> {
    /// Filter and components being checked for
    content: T,
}

// pub fn changed<T: Component + Clone>(
//     query: Query<Ref<T>>,
// ) {   
//     let sample = BevyQueryScope::World;

//     match sample {
//         BevyQueryScope::Entity(entity) => {
//             let Ok(data) =query.get(entity)
//             .inspect_err(|err| warn!("blah blah blah thing doesn't exist")) else {
//                 todo!("something somehting something, tell dioxus that entity is invalid and to requery for entity based on component or something");
//                 (entity, data);
//                 return;
//             }
//         },
//         BevyQueryScope::World => {
//             let datas = query.iter().clone();
//                 todo!("send this to dioxus for it to read inside BevyQuery or somethign and update its state with this component")
//                 datas

//             },
//     }
// }

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // .add_plugins()
        .add_plugins(DioxusInBevyPlugin {})
        .add_plugins(BevyScenePlugin)
        .run();
}
