// use bevy_dioxus_sync::panels::DioxusElementMarker;
use dioxus::prelude::*;
use dioxus_bevy_signals::resource::use_bevy_resource;

use crate::backend::SignDistance;

#[derive(Debug)]
pub struct SignUi;

// impl DioxusElementMarker for SignUi {
//     fn element(&self) -> Element {
//         sign_ui()
//     }
// }

#[component]
pub fn sign_ui() -> Element { 
    
    let sign_distance = use_bevy_resource::<SignDistance>();

    let mut distance_edited = use_signal(|| false);
    let mut distance_str = use_signal(|| "???".to_string());
    
    use_effect(move || {
        if *distance_edited.read() {
            return;
        }
        if let Ok(val) = sign_distance.read_ok(|n| n.0.to_string()) {
            distance_str.set(val);
        }
    });



    let set_distance = move |evt: Event<FormData>| {
        let val = evt.value();
        distance_edited.set(true);
        distance_str.set(val.clone());
        if let Ok(dist) = val.parse::<f32>() {
            sign_distance.mutate(move |n| *n = SignDistance(dist));
        }
    };
    
    rsx! {
        div {
            background_color: "yellow",
            h1 {
                "Second dom!"
            }
            h3 {
                "distance slider: O-----"
            }
            h3 {
                "Alternate Cube Colors:"
            }
            ul {
                li {
                    "Purple"
                }
                li {
                    "Yellow"
                }
            }
            div {
                id: "distance-control",
                label { "Sign Distance:" }
                input {
                    r#type: "number",
                    min: "0.5",
                    max: "5.0",
                    step: "0.1",
                    value: distance_str,
                    oninput: set_distance,
                }
            }
        }

    }
}
