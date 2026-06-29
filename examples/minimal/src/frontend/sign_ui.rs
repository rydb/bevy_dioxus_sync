// use bevy_dioxus_sync::panels::DioxusElementMarker;
use dioxus::prelude::*;

#[derive(Debug)]
pub struct SignUi;

// impl DioxusElementMarker for SignUi {
//     fn element(&self) -> Element {
//         sign_ui()
//     }
// }

#[component]
pub fn sign_ui() -> Element {
    rsx! {
        div {
            class: "sign-content",
            style {
                "display: flex; flex-direction: column; align-items: center; justify-content: center; width: 100%; height: 100%;"
            }
            h1 {
                style { "font-size: 36px; color: #FFFFFF; margin: 0;" }
                "bevy + dioxus"
            }
            p {
                style { "font-size: 16px; color: #AAAAAA; margin: 4px 0 0 0;" }
                "DOM on a 3D surface"
            }
        }
    }
}
