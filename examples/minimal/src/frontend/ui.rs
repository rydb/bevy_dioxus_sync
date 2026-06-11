use std::{cell::Ref, collections::HashMap, ops::Deref, sync::Arc};

use crate::backend::*;
use bevy::sprite_render::MeshMaterial2d;
use bevy_color::{Color, Srgba};
use bevy_dioxus_sync::panels::DioxusElementMarker;
use bevy_ecs::{entity::Entity, query::With};
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use bevy_transform::components::Transform;
use dioxus::prelude::*;
use dioxus_bevy_signals::{asset::{AssetNoneState, use_bevy_asset}, query::{single::use_bevy_single, use_bevy_query}, resource::use_bevy_resource};

#[derive(Debug)]
pub struct AppUi;

impl DioxusElementMarker for AppUi {
    fn element(&self) -> Element {
        app_ui()
    }
}

pub const QUAT_CHAR_INDEX: [&'static str; 4] = ["x", "y", "z", "w"];

#[component]
pub fn app_ui() -> Element {
    let fps = use_bevy_resource::<FPS>();

    let cube = use_bevy_single::<(Entity, &mut Transform, &mut MeshMaterial3d<StandardMaterial>), With<DynamicCube>>();


    let cube_transform = use_memo(move || {
        cube.read_ok(|n| **n.1.read())
    });

    // let cube_transform_str = use_memo(move || {
    //     cube.read_ok(|n| (*n.1.read()).translation.to_string()).unwrap_or_else(|err| err.into())
    // });

    let cube_color_handle = use_memo(move || {
        cube
        .read_ok(|n| Ok(n.2.read().0.id()))
        .unwrap_or_else(|err| Err(AssetNoneState::Error(err.into())))
    });
    let cube_color = use_bevy_asset(cube_color_handle);

    // let cube_color_str = use_memo(move || {
    //     cube_color.read_ok(|n| format!("{:#?}", n.base_color.to_srgba())).unwrap_or_else(|err| err.into())
    // });



    // let cube_rotation_speed = use_bevy_resource::<CubeRotationSpeed>();
    // let cube_translation_speed = use_bevy_resource::<CubeTranslationSpeed>();

    // let rotation_speed_display = use_memo(move || {
    //     cube_rotation_speed.read_ok(|n| n.0.to_string()).unwrap_or_else(|_| "0.0".to_string())
    // });
    // let translation_speed_display = use_memo(move || {
    //     cube_translation_speed.read_ok(|n| n.0.to_string()).unwrap_or_else(|_| "0.0".to_string())
    // });

    // let set_rotation_speed = move |evt: Event<FormData>| {
    //     if let Ok(speed) = evt.value().parse::<f32>() {
    //         cube_rotation_speed.mutate(move |n| *n = CubeRotationSpeed(speed));
    //     }
    // };

    // let set_translation_speed = move |evt: Event<FormData> | {
    //     if let Ok(speed) = evt.value().parse::<f32>() {
    //         cube_translation_speed.mutate(move |n| *n = CubeTranslationSpeed(speed));
    //     }
    // };

    let rgba = use_memo(move || {
        let mut value = Srgba::default();

        if let Ok(color) = &**cube_color.read() {
            value = color.base_color.to_srgba();
        }
        value
    });

    let rgba_css = use_memo(move || {
        let rgba = rgba.read();
        format!(
            "rgba({}, {}, {}, {})",
            (rgba.red * 255.0) as u8,
            (rgba.green * 255.0) as u8,
            (rgba.blue * 255.0) as u8,
            rgba.alpha,
        )
    });

    let value = rsx! {
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
                   b {"example menu "}
                }
            }
            div {
                id: "buttons",
                button {
                    id: "button-red",
                    class: "color-button",
                    onclick: move |_| {
                        cube_color.mutate(|color| *color = StandardMaterial::from_color(Color::srgba(1.0, 0.0, 0.0, 1.0)));
                    },
                }
                button {
                    id: "button-green",
                    class: "color-button",
                    onclick: move |_| {
                        cube_color.mutate(|color| *color = StandardMaterial::from_color(Color::srgba(0.0, 1.0, 0.0, 1.0)))
                    },
                }
                button {
                    id: "button-blue",
                    class: "color-button",
                    onclick: move |_| {
                        cube_color.mutate(|color| *color = StandardMaterial::from_color(Color::srgba(0.0, 0.0, 1.0, 1.0)))
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
                        {cube_transform.read().as_ref().map(|n| n.translation.to_string()).unwrap_or_else(|n| n.clone().into())}
                    }
                }
            }
            // commented out due to font rendeirng bug on nixos when no font is found, TODO: re-add after porting to latest blitz
            // div {
            //     id: "translation-speed-control",
            //     label { "Translation Speed:" }
            //     input {
            //         r#type: "number",
            //         min: "0.0",
            //         max: "10.0",
            //         step: "0.1",
            //         value: translation_speed_display,
            //         oninput: set_translation_speed,
            //     }
            // }
            // div {
            //     // id: "rotation-speed-control",
            //     label { "Rotation Speed:" }
            //     input {
            //         r#type: "number",
            //         min: "0.0",
            //         max: "10.0",
            //         step: "0.1",
            //         value: rotation_speed_display,
            //         oninput: set_rotation_speed,
            //     }
            // }
            div {
                flex: "0 0 150px",
                display: "grid",
                align_items: "center",
                justify_items: "center",
                div {
                    class: "spin-box",
                    background: "{rgba_css}",
                }
            }
            div {
                id: "footer",
                p { "Bevy framerate: {fps}" }
            }
        }
    };
    value
}
