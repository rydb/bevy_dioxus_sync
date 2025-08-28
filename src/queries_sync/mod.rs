use crossbeam_channel::{Receiver, Sender};

// pub mod asset_handle;
pub mod one_component_kind;
pub mod asset_single;

pub struct BevyDioxusIO<B, D = B> {
    pub bevy_tx: Sender<B>,
    pub bevy_rx: Receiver<D>,
    pub dioxus_tx: Sender<D>,
    pub dioxus_rx: Receiver<B>,
}