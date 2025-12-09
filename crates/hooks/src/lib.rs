
use bevy_dioxus_interop::{
    BevyCommandQueueTx, BevyValue,
    signals::CrossDomSignal,
    traits::{BoxGenericDomTypeMap, CrossDomSignalErasedMap},
};
use bevy_ecs::{system::Command, world::CommandQueue};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use dioxus_core::ReactiveContext;
use dioxus_hooks::use_context;
use dioxus_signals::{
    ReadableExt, Signal, WritableExt,
};
use std::fmt::Debug;


// pub mod asset;
// pub mod component;
pub mod resource;

/// hook that handles the logistics of getting a value to and from bevy.
pub fn use_bevy_value<T, V, W, X>(index: V::Index) -> CrossDomSignal<T>
where
    T: Debug + Send + Sync + Clone + 'static,
    V: TransparentWrapper<BoxGenericDomTypeMap<V::Index>> + CrossDomSignalErasedMap + 'static,
    W: Clone + Command + Default + TransparentWrapper<CrossDomSignal<T>>,
    X: Clone + TransparentWrapper<Signal<V>> + 'static,
{
    let command_queue_tx = use_context::<BevyCommandQueueTx>();

    let instance = use_context::<X>();

    let mut binding = TransparentWrapper::peel(instance);
    let mut signals = binding.write();
    let signal = match signals.get_typed::<T>(&index) {
        Some(signal) => signal.clone(),
        None => {
            let signal = request_bevy_signal::<T, V, W>(command_queue_tx);
            signals.insert_signal(signal.clone(), index);
            signal
        }
    };
    signal
}

pub enum SignalStatus<T> {
    Initializing,
    Initialized(T),
}

fn request_bevy_signal<T, U, V>(command_queue_tx: BevyCommandQueueTx) -> CrossDomSignal<T>
where
    T: 'static + Send + Sync + Clone,
    U: TransparentWrapper<BoxGenericDomTypeMap<U::Index>> + CrossDomSignalErasedMap,
    V: Clone + Command + Default + TransparentWrapper<CrossDomSignal<T>>,
{
    let new_signal = CrossDomSignal::new_uninitialized();

    let mut commands = CommandQueue::default();

    let command = V::wrap(new_signal.clone());
    commands.push(command);

    warn!(
        "CURRENT REACTIVE CONTEXT: {}",
        ReactiveContext::current()
            .map(|n| n.to_string())
            .unwrap_or("???".to_string())
    );
    let _ = command_queue_tx
        .0
        .send(commands)
        .inspect_err(|err| warn!("{:#}", err));
    return new_signal;
}
