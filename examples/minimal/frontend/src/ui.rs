use std::fmt::Display;

use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_dioxus_hooks::{asset::use_bevy_component_asset_single, component_single::hook::use_bevy_component_singleton, resource::hook::use_bevy_resource, BevyValue};
use bevy_dioxus_sync::DioxusElementMarker;
use bevy_pbr::prelude::*;
use bevy_transform::components::Transform;
use dioxus::{core::SuperInto, prelude::*};
// use dioxus_signals::*;
// use dioxus_core::Element;
// use dioxus_core_macro::{component, rsx};

#[derive(Component, Clone)]
pub struct DynamicCube;

#[derive(Resource, Clone, Debug, Deref, DerefMut)]
pub struct CubeTranslationSpeed(pub f32);

#[derive(Resource, Debug, Clone)]
pub struct FPS(pub f32);

impl Display for FPS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for CubeTranslationSpeed {
    fn default() -> Self {
        Self(1.0)
    }
}

impl Display for CubeTranslationSpeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Resource, Clone, Debug, Deref, DerefMut)]
pub struct CubeRotationSpeed(pub f32);

impl Display for CubeRotationSpeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for CubeRotationSpeed {
    fn default() -> Self {
        Self(2.0)
    }
}


#[derive(Debug)]
pub struct AppUi;

impl DioxusElementMarker for AppUi {
    fn element(&self) -> Element {
        app_ui()
    }
}

pub const QUAT_CHAR_INDEX: [&'static str; 4] = ["x", "y", "z", "w"];

// static CSS: dioxus::prelude::Asset = asset!("./ui.css");



#[component]
pub fn app_ui() -> Element {
    let fps  = use_bevy_resource::<FPS>();
    let cube_color = use_bevy_component_asset_single::<MeshMaterial3d<StandardMaterial>, _, DynamicCube>();
    // let x = cube_color.peek()
    // let cube_rotation_speed = use_bevy_resource::<CubeRotationSpeed>();
    // let cube_translation_speed = use_bevy_resource::<CubeTranslationSpeed>();
    // let cube_transform = use_bevy_component_singleton::<Transform, DynamicCube>();
    rsx! {
       style { {include_str!("./ui.css")} }
    //     // document::Stylesheet { href: asset!("/src/ui.css") }
    //     div {
    //         id: "panel",
    //         class: "catch-events",
    //         div {
    //             id: "title",
    //             h1 {
    //                u {  
    //                 "bevy_dioxus_sync:"
    //                } 
    //                br {}
    //                {"example menu "}
    //             }
    //         }
            div {
                id: "buttons",
                button {
                    id: "button-red",
                    class: "color-button",
                    onclick: move |_| {
                        cube_color.peek().set_value(StandardMaterial::from_color(Color::srgba(1.0, 0.0, 0.0, 1.0)))
                    },
                }
                button {
                    id: "button-green",
                    class: "color-button",
                    onclick: move |_| {
                        cube_color.peek().set_value(StandardMaterial::from_color(Color::srgba(0.0, 1.0, 0.0, 1.0)))
                    },
                }
                button {
                    id: "button-blue",
                    class: "color-button",
                    onclick: move |_| {
                        cube_color.peek().set_value(StandardMaterial::from_color(Color::srgba(0.0, 0.0, 1.0, 1.0)))
                    },
                }
            }
    //         div {
    //             id: "rotation-display",
    //             label {
    //                 {"Cube Rotation: ".to_string()}
    //             }
    //             label {
    //                 class: "bevy-display",
    //                 {
    //                     let xyzw = &cube_transform.read().read_component().as_ref().map(|n| n.rotation)
    //                     .map(|n| n.to_array())
    //                     .map(|n| {
    //                         n.iter()
    //                         .enumerate()
    //                         .map(|(i, n)| format!("{:#}: {:.2} ", QUAT_CHAR_INDEX[i], n)).collect::<String>()
    //                     }).unwrap_or("???".to_string());

    //                     {xyzw.to_string()}
    //                 }  
    //             }
    //         }
    //         div {
    //             id: "translation-speed-control",
    //             label { "Translation Speed:" }
    //             input {
    //                 r#type: "number",
    //                 min: "0.0",
    //                 max: "10.0",
    //                 step: "0.1",
    //                 value: {
    //                     (&cube_translation_speed.read().read_resource().as_ref().map(|n| format!("{:.2}", n.0)).unwrap_or("???".to_string())).to_string()
    //                 },
    //                 oninput: move |event| {
    //                     if let Ok(speed) = event.value().parse::<f32>() {
    //                         cube_translation_speed.peek().set_resource(CubeTranslationSpeed(speed));
    //                     }
    //                 }
    //             }
    //         }
    //         div {
    //             id: "rotation-speed-control",
    //             label { "Rotation Speed:" }
    //             input {
    //                 r#type: "number",
    //                 min: "0.0",
    //                 max: "10.0",
    //                 step: "0.1",
    //                 value: "{cube_rotation_speed}",
    //                 oninput: move |event| {
    //                     if let Ok(speed) = event.value().parse::<f32>() {
    //                         cube_rotation_speed.peek().set_resource(CubeRotationSpeed(speed));
    //                     }
    //                 }
    //             }
    //         }
            div {
                id: "footer",
                p { "Bevy framerate: {fps}" }
            }
    //     }
    }
}
