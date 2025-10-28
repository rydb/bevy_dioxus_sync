use std::{
    any::{Any, type_name},
    collections::HashMap,
    fmt::Display,
};

use async_std::task::sleep;
use bevy_dioxus_interop::{BevyCommandQueueTx, BevyDioxusIO, InfoPacket, InfoRefershRateMS};
use bevy_ecs::{system::Command, world::CommandQueue};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use dioxus_core::use_hook;
use dioxus_hooks::{try_use_context, use_context, use_future};
use dioxus_signals::{
    Signal, SignalSubscriberDrop, SyncSignal, UnsyncStorage, WritableExt, WriteLock,
};

use tokio::sync::broadcast::{
    self, Receiver as TokioReceiver, Sender as TokioSender
};

pub mod asset;
pub mod component;
pub mod resource;

pub mod traits;
pub use traits::*;

pub enum BevyFetchBackup {
    /// Return value as unknown as it couldn't be fetched
    Unknown,
    /// Return value for when the value exists in bevy, but dioxus hasn't received it yet.
    Uninitialized,
}

impl Default for BevyFetchBackup {
    fn default() -> Self {
        BevyFetchBackup::Unknown
    }
}

impl Display for BevyFetchBackup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            BevyFetchBackup::Unknown => format!("Unable to receive {}", type_name::<Self>()),
            BevyFetchBackup::Uninitialized => "waiting for value from bevy....".to_string(),
        };
        write!(f, "{}", string)
    }
}

pub struct BevyValue<T: Clone + 'static, Index, U> {
    pub(crate) writer: Option<Sender<InfoPacket<T, Index, U>>>,
    pub(crate) reader: Option<TokioReceiver<InfoPacket<T, Index, U>>>,
    pub(crate) value: Result<T, BevyFetchBackup>,
    pub(crate) additional_info: Option<U>,
    pub(crate) index: Option<Index>,
}

impl<T: Clone + 'static, Index: Clone, U: Clone> BevyValue<T, Index, U> {
    pub fn set_value(&mut self, value: T) {
        if let Some(send_channel) = &self.writer {
            let send_result = send_channel
                .send(InfoPacket {
                    update: value.clone(),
                    index: self.index.clone(),
                    additional_info: self.additional_info.clone(),
                })
                .inspect_err(|err| warn!("could not update bevy value signal due to {:#}", err));
            if send_result.is_ok() {
                self.value = Ok(value)
            }
        } else {
            warn!("no send channel for {:#}, skipping", type_name::<T>());
            return;
        }
    }
    pub fn read_value(&self) -> Result<&T, &BevyFetchBackup> {
        self.value.as_ref()
    }
}

impl<T, Index, U> Display for BevyValue<T, Index, U>
where
    T: Display + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Ok(n) => write!(f, "{}", n),
            Err(err) => write!(f, "{}", err),
        }
    }
}

pub type BoxGenericTypeMap<Index> = HashMap<Index, Box<dyn Any + Send + Sync>>;

pub type SignalErasedMapValue<T, Index, AdditionalInfo> =
    SyncSignal<BevyValue<T, Index, AdditionalInfo>>;

pub fn use_bevy_value<T, U, V, W>(
    index: Option<V::Index>,
) -> SyncSignal<BevyValue<T, V::Index, V::AdditionalInfo>>
where
    T: Send + Sync + Clone + 'static,
    U: Clone + 'static + TransparentWrapper<Signal<V>>,
    V: TransparentWrapper<BoxGenericTypeMap<V::Index>> + SignalsErasedMap + 'static,
    W: Command + Default + TransparentWrapper<BevyDioxusIO<T, V::Index, V::AdditionalInfo>>,
{
    let refresh_rate = try_use_context::<InfoRefershRateMS>();
    let command_queue_tx = try_use_context::<BevyCommandQueueTx>();
    let mut signals_register = use_context::<U>();

    let signal = use_hook(|| {
        let mut map_erased: WriteLock<
            '_,
            V,
            UnsyncStorage,
            SignalSubscriberDrop<V, UnsyncStorage>,
        > = TransparentWrapper::peel_mut(&mut signals_register).write();
        let mut value = None;
        if let Some(index) = index.clone() {
            value = map_erased.get_typed::<T>(&index);
        }
        let signal = if let Some(signal) = value {
            signal.clone()
        } else {
            request_bevy_signal::<T, V, W>(
                command_queue_tx,
                map_erased,
                index.clone(),
                W::default(),
            )
        };
        signal
    });
    use_future(move || {
        let mut refresh_rate = refresh_rate.clone();
        let index_known = index.is_some();
        //
        {
            let mut value = signals_register.clone();
            async move {
                let map_erased = TransparentWrapper::peel_mut(&mut value);
                let mut signal = signal.clone();

                let mut reader = {
                    let Some(ref reader) = signal.write().reader else {
                        return;
                    };
                    let reader = reader.resubscribe();
                    reader

                };


                let mut map_erased = map_erased.clone();
                let refresh_rate = refresh_rate.take().unwrap_or_default().0;
                loop {
                    // warn!("looping...");
                    while let Ok(packet) = reader.try_recv().inspect_err(|err| {
                        match err {
                            broadcast::error::TryRecvError::Empty => {},
                            broadcast::error::TryRecvError::Closed => warn!("channel closed for {:#}", type_name::<T>()),
                            broadcast::error::TryRecvError::Lagged(_) => warn!("channel lagging for {:#}", type_name::<T>()),
                        }
                    }) {
                        let mut register_signal = None;
                        if index_known == false {
                            if let Some(index) = packet.index {
                                // warn!("index not received for: {:#?} ", packet);
                                register_signal = Some(index.clone());
                                map_erased.write().insert_typed::<T>(signal.clone(), index);
                            }
                        }
                        let mut asset = signal.write();

                        if let Some(index) = register_signal {
                            asset.index = Some(index.clone());
                        }
                        asset.value = Ok(packet.update);
                    }

                    sleep(std::time::Duration::from_millis(refresh_rate)).await;
                }
            }
        }
    });
    signal
}

fn request_bevy_signal<T, U, V>(
    command_queue_tx: Option<BevyCommandQueueTx>,
    mut signal_registry: WriteLock<'_, U, UnsyncStorage, SignalSubscriberDrop<U, UnsyncStorage>>,
    index: Option<U::Index>,
    request: V,
) -> SyncSignal<BevyValue<T, U::Index, U::AdditionalInfo>>
where
    T: Send + Sync + Clone,
    U: TransparentWrapper<BoxGenericTypeMap<U::Index>> + SignalsErasedMap,
    V: Command + Default + TransparentWrapper<BevyDioxusIO<T, U::Index, U::AdditionalInfo>>,
{
    if let Some(command_queue_tx) = command_queue_tx {
        let channels = V::peel_ref(&request);

        let dioxus_rx = channels.dioxus_rx.resubscribe();
        let dioxus_tx = channels.dioxus_tx.clone();

        let mut commands = CommandQueue::default();
        let command = request;
        commands.push(command);

        //let channels: DioxusTxrX<_> = request.clone().into();
        // let channels = request.txrx();



        let new_signal = SyncSignal::new_maybe_sync(BevyValue {
            value: Err(BevyFetchBackup::Uninitialized),
            reader: Some(dioxus_rx),
            writer: Some(dioxus_tx),
            additional_info: None,
            index: None,
        });
        if let Some(index) = index {
            signal_registry.insert_typed::<T>(new_signal.clone(), index);
        }
        let _ = command_queue_tx
            .0
            .send(commands)
            .inspect_err(|err| warn!("{:#}", err));

        return new_signal;
    } else {
        let new_signal = SyncSignal::new_maybe_sync(BevyValue {
            value: Err(BevyFetchBackup::Unknown),
            reader: None,
            writer: None,
            additional_info: None,
            index: None,
        });
        new_signal
    }
}
