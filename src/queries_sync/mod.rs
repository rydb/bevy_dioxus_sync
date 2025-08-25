use crossbeam_channel::{Receiver, Sender};

// pub mod asset_handle;
pub mod one_component_kind;

pub struct BevyDioxusIO<T> {
    pub dioxus_tx: Sender<T>,
    pub dioxus_rx: Receiver<T>,
    pub bevy_tx: Sender<T>,
    pub bevy_rx: Receiver<T>,
}