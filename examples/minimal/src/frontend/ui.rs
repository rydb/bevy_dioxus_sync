use crate::backend::*;
use bevy_color::Color;
use bevy_dioxus_hooks::{
    asset::use_bevy_component_asset_single,
    component::component_single::hook::use_bevy_component_singleton,
    resource::hook::use_bevy_resource,
};
use bevy_pbr::prelude::*;
use bevy_transform::components::Transform;
use dioxus::prelude::*;

// TODO: uncomment when bevy_dioxus_panels in ready
// #[derive(Debug)]
// pub struct AppUi;

// impl DioxusElementMarker for AppUi {
//     fn element(&self) -> Element {
//         app_ui()
//     }
// }

pub const QUAT_CHAR_INDEX: [&'static str; 4] = ["x", "y", "z", "w"];

#[component]
pub fn app_ui() -> Element {
    let fps = use_bevy_resource::<FPS>();
    let cube_color =
        use_bevy_component_asset_single::<MeshMaterial3d<StandardMaterial>, _, DynamicCube>();
    let cube_rotation_speed = use_bevy_resource::<CubeRotationSpeed>();
    let cube_translation_speed = use_bevy_resource::<CubeTranslationSpeed>();
    let cube_transform = use_bevy_component_singleton::<Transform, DynamicCube>();
    rsx! {
        document::Stylesheet { href: asset!("src/frontend/ui.css") }
        div {
            id: "panel",
            class: "catch-events",
            div {
                id: "title",
                h1 {
                   u {
                    "bevy_dioxus_sync: "
                   }
                   br {}
                   {"example menu "}
                }
            }
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
            div {
                id: "rotation-display",
                label {
                    {"Cube Rotation: ".to_string()}
                }
                label {
                    class: "bevy-display",
                    {
                        let xyzw = &cube_transform.read().read_value().as_ref().map(|n| n.rotation)
                        .map(|n| n.to_array())
                        .map(|n| {
                            n.iter()
                            .enumerate()
                            .map(|(i, n)| format!("{:#}: {:.2} ", QUAT_CHAR_INDEX[i], n)).collect::<String>()
                        }).unwrap_or("???".to_string());

                        {xyzw.to_string()}
                    }
                }
            }
            div {
                id: "translation-speed-control",
                label { "Translation Speed:" }
                input {
                    r#type: "number",
                    min: "0.0",
                    max: "10.0",
                    step: "0.1",
                    value: {
                        (&cube_translation_speed.read().read_value().as_ref().map(|n| format!("{:.2}", n.0)).unwrap_or("???".to_string())).to_string()
                    },
                    oninput: move |event| {
                        if let Ok(speed) = event.value().parse::<f32>() {
                            cube_translation_speed.peek().set_value(CubeTranslationSpeed(speed));
                        }
                    }
                }
            }
            div {
                id: "rotation-speed-control",
                label { "Rotation Speed:" }
                input {
                    r#type: "number",
                    min: "0.0",
                    max: "10.0",
                    step: "0.1",
                    value: "{cube_rotation_speed}",
                    oninput: move |event| {
                        if let Ok(speed) = event.value().parse::<f32>() {
                            cube_rotation_speed.peek().set_value(CubeRotationSpeed(speed));
                        }
                    }
                }
            }
            div {
                id: "footer",
                p { "Bevy framerate: {fps}" }
            }
        }
    }
}
