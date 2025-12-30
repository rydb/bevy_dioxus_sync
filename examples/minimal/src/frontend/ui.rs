use std::collections::HashMap;

use crate::backend::*;
use bevy_dioxus_hooks::{asset::{BevyAssetClone, use_bevy_assets}, query::use_bevy_query, resource::hook::use_bevy_resource};
use bevy_dioxus_interop::signals::CrossDomSignal;
use bevy_dioxus_sync::panels::DioxusElementMarker;
use bevy_ecs::entity::Entity;
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use bevy_transform::components::Transform;
use dioxus::prelude::*;

#[derive(Debug)]
pub struct AppUi;

impl DioxusElementMarker for AppUi {
    fn element(&self) -> Element {
        app_ui()
    }
}

pub const QUAT_CHAR_INDEX: [&'static str; 4] = ["x", "y", "z", "w"];

// fn get_handle()

#[component]
pub fn app_ui() -> Element {
    let fps = use_bevy_resource::<FPS>();
    let transforms = use_bevy_query::<(Entity, &Transform), ()>();

    let mats_signal = use_bevy_assets::<StandardMaterial>();

    let handle = if let Ok(colors) =  use_bevy_query::<(Entity, &MeshMaterial3d<StandardMaterial>), ()>().get() 
    && let Some((e, (_, color))) = colors.iter().last()
    && let Ok(handle) = color.0.get()
    {
        if let Ok(mats) = mats_signal.get() 
        && let Some(_handle) = mats.get(&handle) {

        } else {
            let mut map = HashMap::new();
            map.insert(handle.0.clone(), CrossDomSignal::new(BevyAssetClone::Loading(handle.0.clone())));
            let _ = mats_signal.set(map);
        }
        Some(handle.0.clone())
    } else {
        None
    };
    
    let color = if let Ok(colors) = mats_signal.get() 
    && let Some(handle) = handle
    && let Some(color_signal) = colors.get(&handle)
    && let Ok(color_state) = color_signal.get() {
        Some(color_state)
    } else {
        None
    };

    let color_str = color.map(|n| n.get().map(|n| format!("{:#?}", n.base_color.to_srgba())).unwrap_or("???".to_owned()) ).unwrap_or("???".to_string());
    

    let mut transform_list = Vec::new();

    match transforms.get() {
        Ok(transforms) => {
            for (_, (e, transform)) in transforms.iter() {
                transform_list.push(transform.0.get().map(|n| n.translation))
            }
        },
        Err(_) => {},
    }
    let transform_string = format!("{:#?}", transform_list);



    // for (_, (e, color)) in colors.get().unwrap().iter() {
    //     color_list.push(color.0.get().unwrap().0.clone())
    // }
    // let mut cube_color =
    //     use_bevy_component_asset_single::<MeshMaterial3d<StandardMaterial>, _, DynamicCube>();
    // let mut cube_rotation_speed = use_bevy_resource::<CubeRotationSpeed>();
    // let mut cube_translation_speed = use_bevy_resource::<CubeTranslationSpeed>();
    // let cube_transform = use_bevy_component_singleton::<Transform, DynamicCube>();

    // let color = cube_color
    //     .read()
    //     .read_value()
    //     .map(|n| n.base_color)
    //     .unwrap_or(Color::default())
    //     .to_srgba()
    //     .to_f32_array();
    // let [r, g, b, a] = color.map(|c| (c * 255.0) as u8);

    // rsx! {
    //     document::Stylesheet { href: asset!("src/frontend/ui.css") }
    //     div {
    //         id: "panel",
    //         class: "catch-events",
    //         div {
    //             id: "title",
    //             h1 {
    //                u {
    //                 "bevy_dioxus_sync: "
    //                }
    //                br {}
    //                b {"example menu "}
    //             }
    //         }
    //         div {
    //             id: "buttons",
    //             button {
    //                 id: "button-red",
    //                 class: "color-button",
    //                 onclick: move |_| {
    //                     cube_color.write().set_value(StandardMaterial::from_color(Color::srgba(1.0, 0.0, 0.0, 1.0)))
    //                 },
    //             }
    //             button {
    //                 id: "button-green",
    //                 class: "color-button",
    //                 onclick: move |_| {
    //                     cube_color.write().set_value(StandardMaterial::from_color(Color::srgba(0.0, 1.0, 0.0, 1.0)))
    //                 },
    //             }
    //             button {
    //                 id: "button-blue",
    //                 class: "color-button",
    //                 onclick: move |_| {
    //                     cube_color.write().set_value(StandardMaterial::from_color(Color::srgba(0.0, 0.0, 1.0, 1.0)))
    //                 },
    //             }
    //         }
    //         div {
    //             id: "rotation-display",
    //             label {
    //                 {"Cube Rotation: ".to_string()}
    //             }
    //             label {
    //                 class: "bevy-display",
    //                 {
    //                     let xyzw = &cube_transform.read().read_value().map(|n| n.rotation)
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
    //                     (&cube_translation_speed.read().read_value().map(|n| format!("{:.2}", n.0)).unwrap_or("???".to_string())).to_string()
    //                 },
    //                 oninput: move |event| {
    //                     if let Ok(speed) = event.value().parse::<f32>() {
    //                         cube_translation_speed.write().set_value(CubeTranslationSpeed(speed));
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
    //                         cube_rotation_speed.write().set_value(CubeRotationSpeed(speed));
    //                     }
    //                 }
    //             }
    //         }
    //         div {
    //             flex: "0 0 150px",
    //             display: "grid",
    //             align_items: "center",
    //             justify_items: "center",
    //             div {
    //                 class: "spin-box",
    //                 background: "rgba({r}, {g}, {b}, {a}",
    //             }
    //         }
    //         div {
    //             id: "footer",
    //             p { "Bevy framerate: {fps}" }
    //         }
    //     }s
    // }
    rsx! {
        document::Stylesheet { href: asset!("src/frontend/ui.css") }

        div {
            h1 {"this is rendering!"}
            h1 {{format!("Bevy framerate: {:#}", fps)}}
        }
        div {
            h1 {
                "bevy query results:"
            }
            h1 {
                {
                    transform_string
                }
            }
            h1 {
                "bevy colors:"
            }
            h1 {
                {color_str}
            }
        }
    }
}
