use std::{
    any::{Any, type_name},
    collections::HashMap,
    fmt::Display,
};

use async_std::task::sleep;
use bevy_dioxus_interop::{
    BevyCommandQueueTx, BevyDioxusIO, BevyDioxusPacket, BevyFetchBackup, BevyValue, InfoPacket, InfoRefershRateMS, InfoUpdate, StatusUpdate, signals::CrossDomSignal, traits::{BoxGenericDomTypeMap, CrossDomSignalErasedMap, ErasedSignal}
};
use bevy_ecs::{system::Command, world::CommandQueue};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use crossbeam_channel::Sender;
use dioxus_core::{ReactiveContext, use_hook};
use dioxus_hooks::{try_use_context, use_context, use_future, use_memo, use_signal, use_signal_sync};
use dioxus_signals::{
    Memo, ReadSignal, Readable, ReadableExt, Signal, SignalSubscriberDrop, SyncSignal, SyncStorage, UnsyncStorage, WritableExt, WriteLock
};
use std::fmt::Debug;

use tokio::sync::broadcast::{self, Receiver as TokioReceiver};

// pub mod asset;
// pub mod component;
pub mod resource;

/// hook that handles the logistics of getting a value to and from bevy.
pub fn use_bevy_value<T, V, W, X>(
    index: V::Index,
) -> CrossDomSignal<T> 
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
        Some(signal) => {
            println!("outputing current signal state");
            signal.clone()
        },
        None => {
            println!("requesting signal..");

            let signal = request_bevy_signal::<T, V, W>(command_queue_tx);
            signals.insert_signal(signal.clone(), index);
            signal

        },
    };
    signal
}

pub enum SignalStatus<T> {
    Initializing,
    Initialized(T)
}

fn request_bevy_signal<T, U, V>(
    command_queue_tx: BevyCommandQueueTx,
) -> CrossDomSignal<T>
where
    T: 'static + Send + Sync + Clone,
    U: TransparentWrapper<BoxGenericDomTypeMap<U::Index>> + CrossDomSignalErasedMap,
    V: Clone + Command + Default + TransparentWrapper<CrossDomSignal<T>>,
{
    let new_signal = CrossDomSignal::new_uninitialized();

    let mut commands = CommandQueue::default();

    let command = V::wrap(new_signal.clone());
    commands.push(command);

    let _ = command_queue_tx
        .0
        .send(commands)
        .inspect_err(|err| warn!("{:#}", err));
    return new_signal;
    
}
