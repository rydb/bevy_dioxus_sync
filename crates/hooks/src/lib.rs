use std::{any::{type_name, Any, TypeId}, collections::HashMap, fmt::Display, hash::Hash, ops::Deref};

use async_std::task::sleep;
use bevy_dioxus_interop::{BevyCommandQueueTx, BevyDioxusIO, BevyTxChannel, BoxAnyTypeMap, InfoPacket, InfoRefershRateMS};
use bevy_ecs::{system::Command, world::CommandQueue};
use bevy_log::{tracing::Value, warn};
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use dioxus_core::use_hook;
use dioxus_hooks::{try_use_context, use_context, use_future};
use dioxus_signals::{Signal, SignalSubscriberDrop, SyncSignal, SyncStorage, UnsyncStorage, WritableExt, WriteLock};
use async_std::sync::Arc;
use crate::resource::hook::{ResourceRegistry, BevyResources};

// pub mod asset_handle;
// pub mod asset_single;
pub mod asset;
pub mod component_single;
pub mod resource;

pub mod traits;


/// What dioxus shows incase the unerlying can't be fetched.
pub enum BevyFetchBackup {
    /// Return value as unknown as it couldn't be fetched
    Unknown,
    /// Return lorem ipsum block
    LoremIpsum,
    /// Return value for when the value exists in bevy, but dioxus hasn't received it yet.
    Uninitialized,
}

impl Default for BevyFetchBackup {
    fn default() -> Self {
        BevyFetchBackup::Unknown
    }
}

impl Display for BevyFetchBackup{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            BevyFetchBackup::Unknown => "Unable to receive value",
            // todo: Implement this properly.
            BevyFetchBackup::LoremIpsum => "Lorem Ipsum",
            BevyFetchBackup::Uninitialized => "waiting for value from bevy....",
        };
        write!(f, "{}", string)
    }
}

pub struct BevyValue<T: Clone + 'static, Index, U> {
    pub(crate) writer: Option<Sender<InfoPacket<T, Index, U>>>,
    pub(crate) reader: Option<Receiver<InfoPacket<T, Index, U>>>,
    pub(crate) value: Result<T, BevyFetchBackup>,
    pub(crate) additional_info: Option<U>,
    pub(crate) index: Option<Index>
}

impl<T: Clone + 'static, Index: Clone, U: Clone> BevyValue<T, Index, U> {
    pub fn set_value(&self, value: T) {
        if let Some(send_channel) = &self.writer {
            let _ = send_channel.send(
                InfoPacket { update: value.clone(), index: self.index.clone(), additional_info: self.additional_info.clone() }
            
            )
            .inspect_err(|err| warn!("could not update bevy value signal due to {:#}", err));
        } else {
            warn!("no send channel for {:#}, skipping", type_name::<T>());
            return
        }
    }
    pub fn read_value(&self) -> &Result<T, BevyFetchBackup> {
        &self.value
    }
}

impl<T, Index, U> Display for BevyValue<T, Index, U>
    where
        T: Display + Clone
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Ok(n) => write!(f, "{}", n),
            Err(err) => write!(f, "{}", err),
        }
    }
}

pub type BoxGenericTypeMap<Index> = HashMap<Index, Box<dyn Any + Send + Sync>>;

// pub trait SignalsErasedMap 
//     where
//         Self: TransparentWrapper<BoxGenericTypeMap<Self::Index>> + Sized,
// {
//     type Value<T: Clone + Send + Sync + 'static>: Clone + 'static + Send + Sync;
//     type Index: Hash + Eq + Clone + Send + Sync;
//     fn insert_typed<T: Clone + Send + Sync + 'static>(&mut self, value: Self::Value::<T>, index: Self::Index)
//     {
//         let map = TransparentWrapper::peel_mut(self);
//         let erased = Box::new(value);
//         map.insert(index, erased);
//     }

//     fn get_typed<T: Clone + Send + Sync + 'static>(&mut self, index: &Self::Index) -> Option<&mut Self::Value<T>>
//     {
//         let map = TransparentWrapper::peel_mut(self);

//         let value = map.get_mut(&index)?;

//         value.downcast_mut::<Self::Value<T>>()
//     }
//     // fn extend(&mut self, value: Self) {
//     //     let map = TransparentWrapper::peel_mut(self);
//     //     let value = TransparentWrapper::peel(value);
//     //     map.extend(value);
//     // }        
// }

pub type SignalErasedMapValue<T, Index, AdditionalInfo> = SyncSignal<BevyValue<T, Index, AdditionalInfo>>;

pub trait SignalsErasedMap 
    where
        Self: TransparentWrapper<BoxGenericTypeMap<Self::Index>> + Sized,
{
    // type Value: Clone + 'static + Send + Sync;
    type Index: Hash + Eq + Clone + Send + Sync + 'static;
    type AdditionalInfo: Send + Sync + 'static;
    fn insert_typed<T: Clone + Send + Sync + 'static>(&mut self, value: SignalErasedMapValue<T, Self::Index, Self::AdditionalInfo>, index: Self::Index)
    {
        let map = TransparentWrapper::peel_mut(self);
        let erased = Box::new(value);
        map.insert(index, erased);
    }

    fn get_typed<T: Clone + Send + Sync + 'static>(&mut self, index: &Self::Index) -> Option<&mut SignalErasedMapValue<T, Self::Index, Self::AdditionalInfo>>
    {
        let map = TransparentWrapper::peel_mut(self);

        let value = map.get_mut(&index)?;

        value.downcast_mut::<SignalErasedMapValue<T, Self::Index, Self::AdditionalInfo>>()
    }
}

pub fn use_bevy_value<T, U, V, W>(index: Option<V::Index>) -> SyncSignal<BevyValue<T, V::Index, V::AdditionalInfo>> 
    where
        T: Send + Sync + Clone + 'static,
        U: Clone + 'static + TransparentWrapper<Signal<V>>,
        V: TransparentWrapper<BoxGenericTypeMap<V::Index>> + SignalsErasedMap + 'static,
        W: Clone + Command + Default + TransparentWrapper<BevyDioxusIO<T, V::Index, V::AdditionalInfo>>,
{
    let refresh_rate = try_use_context::<InfoRefershRateMS>();
    let command_queue_tx = try_use_context::<BevyCommandQueueTx>();
    let mut signals_register = use_context::<U>();

    let signal = use_hook(|| {
        let mut map_erased: WriteLock<'_, V, UnsyncStorage, SignalSubscriberDrop<V, UnsyncStorage>> = TransparentWrapper::peel_mut(&mut signals_register).write();
        let mut value = None;
        if let Some(index) = index.clone() {
            value = map_erased.get_typed::<T>(&index);
        } 
        let signal = if let Some(signal) = value {
            signal.clone()
        } else {
            request_bevy_signal::<T, V, W>(command_queue_tx, map_erased, index.clone(),W::default())
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
            let mut signal =
                signal.clone();
            let Some(reader) = signal.clone().write().reader.clone() else {
                return
            };
            let mut map_erased = map_erased.clone();
            let refresh_rate = refresh_rate.take().unwrap_or_default().0;
            loop {
                while let Ok(packet) = reader.try_recv() {
                    let mut register_signal = None;
                    if index_known == false {
                        if let Some(index) = packet.index {
                            register_signal = Some(index.clone());
                            map_erased.write().insert_typed::<T>(signal.clone(), index);
                        }
                    }
                    let mut asset = signal.write();

                    if let Some(index) = register_signal {
                        warn!("updated index for: {:#}", type_name::<V>());
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


type RequestA<T> = <T as DioxusTxRx>::A;
type AdditionalInfo<T> = Option<<T as DioxusTxRx>::AdditionalInfo>;
type Index<T> = T;

type Channels<T> = BevyDioxusIO<RequestA<T>, Index<T>, AdditionalInfo<T>>;


pub trait DioxusTxRx
    where
        Self:
            TransparentWrapper<Channels<Self>> + Sized
{
    type A: Send + Sync + Clone + 'static;
    type AdditionalInfo: Send + Sync;
    type Index: Send + Sync;
}

pub enum IndexInitialized {
    No,
    Yes,
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
        V: Clone + Command + Default + TransparentWrapper<BevyDioxusIO<T, U::Index, U::AdditionalInfo>>,
{
    warn!("made it to request signal");
    if let Some(command_queue_tx) = command_queue_tx {
        let mut commands = CommandQueue::default();

        //let channels: DioxusTxrX<_> = request.clone().into();
        let channels = V::peel(request.clone());
        // let channels = request.txrx();
        let command = request;

        let dioxus_rx = channels.dioxus_rx.clone();
        let dioxus_tx = channels.dioxus_tx.clone();
        commands.push(command);

        let new_signal = SyncSignal::new_maybe_sync(BevyValue {
            value: Err(BevyFetchBackup::Uninitialized),
            reader: Some(dioxus_rx),
            writer: Some(dioxus_tx),
            additional_info: None,
            index: None
        });
        if let Some(index) = index {
            signal_registry.insert_typed::<T>(new_signal.clone(), index);

        }
        let _ = command_queue_tx.0
            .send(commands)
            .inspect_err(|err| warn!("{:#}", err));
        warn!("finished request signal in known state");

        return new_signal;
    } else {
        let new_signal = SyncSignal::new_maybe_sync(BevyValue {
            value: Err(BevyFetchBackup::Unknown),
            reader: None,
            writer: None,
            additional_info: None,
            index: None,
        });
        warn!("finished request signal in unknown state");

        new_signal
    }

}