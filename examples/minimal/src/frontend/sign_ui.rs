use bevy_color::Color;
use bevy_ecs::{entity::Entity, query::With};
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
// use bevy_dioxus_sync::panels::DioxusElementMarker;
use dioxus::prelude::*;
use dioxus_bevy_signals::{asset::{AssetNoneState, use_bevy_asset}, query::{single::use_bevy_single}, resource::use_bevy_resource};
use dioxus_bevy_signals::macros::debug;

use crate::backend::{DynamicCube, SignDistance};

#[derive(Debug)]
pub struct SignUi;

const DISTANCE_INCREMENT: f32 = 1.0;
#[component]
pub fn sign_ui() -> Element {
    let cube_distance = use_bevy_resource::<SignDistance>();
    let cube = use_bevy_single::<(Entity, &mut MeshMaterial3d<StandardMaterial>), With<DynamicCube>>();

    #[allow(unused)]
    let _cube_db = use_memo(move || {
        let r = cube.read_ok(|n| n.1.read().0.id()).map_err(|err| format!("{:?}", err));
        debug!("sign_ui: color_handle={:?}", r);
    });

    let cube_color_handle = use_memo(move || {
        cube.read_ok(|n| n.1.read().0.id()).map_err(|err| AssetNoneState::Error(err.into()))
    });
    let cube_color = use_bevy_asset(cube_color_handle);


    let increment = move |_evt| {
        cube_distance.mutate(|n| n.0 += DISTANCE_INCREMENT);
    };

    let decrement = move |_evt | {
        cube_distance.mutate(|n| n.0 -= DISTANCE_INCREMENT);
    };

    rsx! {
        div {
            id: "panel",
            class: "catch-events",
            document::Stylesheet { href: asset!("src/frontend/ui.css") },
            h1 {
                "world space dom"
            }
            div {
                id: "distance-control",
                label { "Cube Distance:" }
                div {
                    class: "stepper-row",
                    button {
                        class: "stepper-btn",
                        onclick: decrement,
                        "-"
                    }
                    span {
                        class: "stepper-value",
                        "{cube_distance}"
                    }
                    button {
                        class: "stepper-btn",
                        onclick: increment,
                        "+"
                    }
                }
            }
            h3 {
                "Alternate Cube Colors:"
            }
           div {
                id: "buttons",
                button {
                    background: "purple",
                    class: "color-button",
                    onclick: move |_| {
                        cube_color.mutate(|color| *color = StandardMaterial::from_color(Color::srgba(0.502, 0.0, 0.502, 1.0)))
                    },
                }
                button {
                    background: "yellow",
                    class: "color-button",
                    onclick: move |_| {
                        cube_color.mutate(|color| *color = StandardMaterial::from_color(Color::srgba(1.0, 1.0, 0.0, 1.0)))
                    },
                }
            }
        }

    }
}
