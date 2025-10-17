use std::{any::{type_name, Any, TypeId}, collections::HashMap, fmt::Display, hash::Hash, ops::Deref};

use async_std::task::sleep;
use bevy_dioxus_interop::{BevyCommandQueueTx, BevyDioxusIO, BoxAnyTypeMap, InfoRefershRateMS};
use bevy_ecs::{system::Command, world::CommandQueue};
use bevy_log::{tracing::Value, warn};
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use dioxus_core::use_hook;
use dioxus_hooks::{try_use_context, use_context, use_future};
use dioxus_signals::{Signal, SignalSubscriberDrop, SyncSignal, SyncStorage, UnsyncStorage, WritableExt, WriteLock};

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

pub struct BevyValue<T: Clone + 'static, U = ()> {
    pub(crate) writer: Option<Sender<(T, Option<U>)>>,
    pub(crate) reader: Option<Receiver<(T, Option<U>)>>,
    pub(crate) value: Result<T, BevyFetchBackup>,
    pub(crate) additional_info: Option<U>
}

impl<T: Clone> BevyValue<T> {
    pub fn set_value(&self, value: T) {
        if let Some(send_channel) = &self.writer {
            send_channel.send((value.clone(), self.additional_info))
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

impl<T> Display for BevyValue<T>
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

pub trait SignalsErasedMap 
    where
        Self: TransparentWrapper<BoxGenericTypeMap<Self::Index>> + Sized,
{
    type Value<T: Clone + Send + Sync + 'static>: Clone + 'static + Send + Sync;
    type Index: Hash + Eq;
    fn insert_typed<T: Clone + Send + Sync + 'static>(&mut self, value: Self::Value::<T>, index: Self::Index)
    {
        let map = TransparentWrapper::peel_mut(self);
        let erased = Box::new(value);
        map.insert(index, erased);
    }

    fn get_typed<T: Clone + Send + Sync + 'static>(&mut self, index: &Self::Index) -> Option<&mut Self::Value<T>>
    {
        let map = TransparentWrapper::peel_mut(self);

        let value = map.get_mut(&index)?;

        value.downcast_mut::<Self::Value<T>>()
    }
    // fn extend(&mut self, value: Self) {
    //     let map = TransparentWrapper::peel_mut(self);
    //     let value = TransparentWrapper::peel(value);
    //     map.extend(value);
    // }        
}

// impl SyncSignalErasedMap {
//     type Generic<T>;
//     type Index;
//     fn insert<T: 'static + Send + Sync + Clone>(&mut self, value: Self::Generic::<T>)
//     {
//         let map = TransparentWrapper::peel_mut(self);
//         let erased = Box::new(value);
//         map.insert(TypeId::of::<T>(), erased);
//     }

//     fn get<T: 'static + Clone>(&mut self) -> Option<&mut SyncSignal<BevyValue<T>>>
//     {
//         let map = TransparentWrapper::peel_mut(self);

//         let value = map.get_mut(&TypeId::of::<T>())?;

//         value.downcast_mut::<SyncSignal<BevyValue<T>>>()
//     }
//     fn extend(&mut self, value: Self) {
//         let map = TransparentWrapper::peel_mut(self);
//         let value = TransparentWrapper::peel(value);
//         map.extend(value);
//     }    
// }


pub fn use_bevy_value<T, U, V, W, X>(index: V::Index) -> SyncSignal<BevyValue<T, X>> 
    where
        T: Send + Sync + Clone + 'static,
        U: Clone + 'static + TransparentWrapper<Signal<V>>,
        V: 'static + SignalsErasedMap<Value<T> = SyncSignal<BevyValue<T, X>>>,
        W: Clone + Command + Default + DioxusTxRx<T, X>,
        X: Send + Sync + Clone
{
    let refresh_rate = try_use_context::<InfoRefershRateMS>();

    let command_queue_tx = try_use_context::<BevyCommandQueueTx>();
    let mut signals_register = use_context::<U>();

    let signal = use_hook(|| {
        let mut map_erased: WriteLock<'_, V, UnsyncStorage, SignalSubscriberDrop<V, UnsyncStorage>> = TransparentWrapper::peel_mut(&mut signals_register).write();
        //let signals_list = V::peel_mut(&mut map_erased);
        // let mut map_erased = TransparentWrapper::peel(signals_register).write();
        let value = map_erased.get_typed::<T>(&index);
        let signal = if let Some(signal) = value {
            signal.clone()
        } else {
            request_bevy_signal::<T, V, W, X>(command_queue_tx, map_erased, index,W::default())
            //request_resource_signal::<T>(command_queue_tx, map_erased.0)
        };
        signal
    });
    use_future(move || {
        let mut refresh_rate = refresh_rate.clone();
        async move {
            let mut signal =
                signal.clone();
            let Some(reader) = signal.clone().write().reader.clone() else {
                return
            };
            let refresh_rate = refresh_rate.take().unwrap_or_default().0;
            loop {
                while let Ok((value, additional_info)) = reader.try_recv() {
                    let mut asset = signal.write();
                    asset.value = Ok(value)
                }
                sleep(std::time::Duration::from_millis(refresh_rate)).await;
            }
        }
    });
    signal
}


pub trait DioxusTxRx<T, AdditionalInfo = (), U = T> {

    fn txrx(&self) -> BevyDioxusIO<(T, Option<AdditionalInfo>), (U, Option<AdditionalInfo>)>;
}

fn request_bevy_signal<T, U, V, W>
    
(
    command_queue_tx: Option<BevyCommandQueueTx>,
    mut signal_registry: WriteLock<'_, U, UnsyncStorage, SignalSubscriberDrop<U, UnsyncStorage>>,
    // mut signal_registry: U,
    index: U::Index,
    //signal_registry: &mut BoxAnyTypeMap,
    request: V,
) -> SyncSignal<BevyValue<T, W>> 
    where
        T: Send + Sync + Clone,
        U: SignalsErasedMap<Value<T> = SyncSignal<BevyValue<T, W>>>,
        V: Clone + Command + Default + DioxusTxRx<T, W>, //Into<DioxusTxrX<T>>
        W: Send + Sync + Clone
{

    if let Some(command_queue_tx) = command_queue_tx {
        let mut commands = CommandQueue::default();

        //let channels: DioxusTxrX<_> = request.clone().into();
        let channels = request.txrx();
        let command = request;

        let dioxus_rx = channels.dioxus_rx.clone();
        let dioxus_tx = channels.dioxus_tx.clone();
        commands.push(command);

        let new_signal = SyncSignal::new_maybe_sync(BevyValue {
            value: Err(BevyFetchBackup::Uninitialized),
            reader: Some(dioxus_rx),
            writer: Some(dioxus_tx),
            additional_info: None,
        });
        // let signal_registry = U::peel_mut(&mut signal_registry);
        signal_registry.insert_typed::<T>(new_signal.clone(), index);
        let _ = command_queue_tx.0
            .send(commands)
            .inspect_err(|err| warn!("{:#}", err));

        return new_signal;
    } else {
        let new_signal = SyncSignal::new_maybe_sync(BevyValue {
            value: Err(BevyFetchBackup::Unknown),
            reader: None,
            writer: None,
            additional_info: None,
        });
        new_signal
    }


}