use std::any::{type_name, TypeId};

use async_std::task::sleep;
use bevy_log::warn;
use bevy_platform::collections::HashMap;
use crossbeam_channel::{Receiver, Sender};
use dioxus::prelude::*;
use dioxus_bevy_panel::{dioxus_in_bevy_plugin::DioxusProps, DioxusRxChannelsUntyped, DioxusTxChannelsUntyped, ErasedSubGenericMap};

use crate::bevy_scene_plugin::CubeRotationSpeed;

macro_rules! define_ui_state {
    (
        $($field:ident : $type:ty = $default:expr),* $(,)?
    ) => { paste! {
        #[allow(dead_code)]
        #[derive(Clone, Copy, Debug)]
        pub struct UiState {
            $($field: Signal<$type>,)*
        }

        #[allow(dead_code)]
        impl UiState {
            fn default() -> Self {
                Self {
                    $($field: Signal::new($default),)*
                }
            }

            $(pub const [<DEFAULT_ $field:upper>]: $type = $default;)*
        }

        #[allow(dead_code)]
        #[derive(Debug)]
        pub enum UIMessage {
            $([<$field:camel>]($type),)*
        }
    }};
}

define_ui_state! {
    cube_color: [f32; 4] = [0.0, 0.0, 1.0, 1.0],
    cube_translation_speed: f32 = 2.0,
    //cube_rotation_speed: f32 = 1.0,
    fps: f32 = 0.0,
}



pub fn app_ui() -> Element {

    let mut registers = use_context::<UiRegisters>();
    let mut state = use_context::<UiState>();

    //let mut rotation_speed = use_context::<CubeRotationSpeed>();

    // use_effect({
    //     let ui_sender = props.ui_sender.clone();
    //     move || {
    //         println!("Color changed to {:?}", state.cube_color);
    //         ui_sender
    //             .send(UIMessage::CubeColor((state.cube_color)()))
    //             .unwrap();
    //     }
    // });

    // use_effect({
    //     let ui_sender = props.ui_sender.clone();
    //     move || {
    //         println!("Rotation speed changed to {:?}", state.cube_rotation_speed);
    //         ui_sender
    //             .send(UIMessage::CubeRotationSpeed((state.cube_rotation_speed)()))
    //             .unwrap();
    //     }
    // });

    // use_effect({
    //     let ui_sender = props.ui_sender.clone();
    //     move || {
    //         println!(
    //             "Translation speed changed to {:?}",
    //             state.cube_translation_speed
    //         );
    //         ui_sender
    //             .send(UIMessage::CubeTranslationSpeed((state
    //                 .cube_translation_speed)(
    //             )))
    //             .unwrap();
    //     }
    // });

    use_future(move || {
        async move {
            loop {
                let bevy_receiver = registers.dioxus_rx_registry.write().0.get::<UIMessage>().clone();

                // // Update UI every 1s in this demo.
                // sleep(std::time::Duration::from_millis(1000)).await;
                sleep(std::time::Duration::from_millis(100)).await;

                let mut fps = Option::<f32>::None;

                let Some(ref app_receiver) = bevy_receiver else {
                    warn!("no app receiver");
                    sleep(std::time::Duration::from_millis(1000)).await;
                    continue;
                };
                warn!("attempting to recieve message");
                while let Ok(message) = app_receiver.clone().try_recv().inspect_err(|err| warn!("couldn't recieve, reason: {:#}", err)) {
                    if let UIMessage::Fps(v) = message {
                        warn!("fps set to {:#}", v);
                        fps = Some(v)
                    }
                } 

                if let Some(fps) = fps {
                    println!("fps set to {:#}", fps);
                    state.fps.set(fps);
                }
            }
        }
    });

    rsx! {
        style { {include_str!("./ui.css")} }
        // div {
        //     id: "panel",
        //     p {"success!" }
        // }
        div {
            id: "panel",
            class: "catch-events",
            div {
                id: "title",
                h1 { "Dioxus In Bevy Example" }
            }
            div {
                id: "buttons",
                button {
                    id: "button-red",
                    class: "color-button",
                    onclick: move |_| state.cube_color.set([1.0, 0.0, 0.0, 1.0]),
                }
                button {
                    id: "button-green",
                    class: "color-button",
                    onclick: move |_| state.cube_color.set([0.0, 1.0, 0.0, 1.0]),
                }
                button {
                    id: "button-blue",
                    class: "color-button",
                    onclick: move |_| state.cube_color.set([0.0, 0.0, 1.0, 1.0]),
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
                    value: "{state.cube_translation_speed}",
                    oninput: move |event| {
                        if let Ok(speed) = event.value().parse::<f32>() {
                            state.cube_translation_speed.set(speed);
                        }
                    }
                }
            }
        // div {
        //     id: "rotation-speed-control",
        //     label { "Rotation Speed:" }
        //     input {
        //         r#type: "number",
        //         min: "0.0",
        //         max: "10.0",
        //         step: "0.1",
        //         value: "{rotation_speed.0}",
        //         oninput: move |event| {
        //             if let Ok(speed) = event.value().parse::<f32>() {
        //                 //state.cube_rotation_speed.set(speed);
        //                 rotation_speed.0 = speed
        //             }
        //         }
        //     }
        // }
            div {
                id: "footer",
                p { "Bevy framerate: {state.fps}" }
            }
        }
    }
}
