use std::{any::{type_name, TypeId}, collections::HashMap, fmt::Display, ops::Deref};

use bevy_dioxus_interop::{BevyCommandQueueTx, BoxAnyTypeMap};
use bevy_ecs::{system::Command, world::CommandQueue};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use dioxus_signals::{Signal, SignalSubscriberDrop, SyncSignal, SyncStorage, UnsyncStorage, WriteLock};

use crate::resource::hook::ResourcesErased;

// pub mod asset_handle;
pub mod asset_single;
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

pub struct BevyValue<T: Clone + 'static> {
    pub(crate) writer: Option<Sender<T>>,
    pub(crate) reader: Option<Receiver<T>>,
    //receiver: Receiver<T>,
    pub(crate) value: Result<T, BevyFetchBackup>,
}

impl<T: Clone> BevyValue<T> {
    pub fn set_value(&self, value: T) {
        if let Some(send_channel) = &self.writer {
            send_channel.send(value.clone())
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

pub trait BevyValueNewType {}

pub enum BevyValueKind{
    Resource
}

// pub trait BevyRegistryKindErased 
//     where
//         Self: TransparentWrapper<BoxAnyTypeMap>
// {
//     type Generic<T: BevyValueNewType + Clone + Send + Sync + 'static>: Send + Sync + 'static;

//     fn insert<T: Clone + Send + Sync + 'static>(&mut self, value: Self::Generic<T>) {
//         let map = TransparentWrapper::peel_mut(self);
//         let erased = Box::new(value);
//         map.insert(TypeId::of::<T>(), erased);
//     }

//     fn get<T: Clone + Send + Sync + 'static>(
//         &mut self,
//     ) -> Option<&mut Self::Generic<T>> {
//         let map = TransparentWrapper::peel_mut(self);

//         let value = map.get_mut(&TypeId::of::<T>())?;

//         value.downcast_mut::<Self::Generic<T>>()
//     }
//     fn extend(&mut self, value: Self) {
//         let map = TransparentWrapper::peel_mut(self);
//         let value = TransparentWrapper::peel(value);
//         map.extend(value);
//     }
// }

pub struct DioxusTxrX<T> {
    dioxus_tx: Sender<T>,
    dioxus_rx: Receiver<T>,
}

pub trait BevyRegistryKindErased<T>
    where
        Self: TransparentWrapper<BoxAnyTypeMap>
{
    type RequestKind: Clone + Command + Into<DioxusTxrX<T>>;
    // fn insert<T: Clone>(&mut self, signal: Signal<BevyValue<T>, SyncStorage>);
}
fn request_bevy_signal<T: Clone + Send + Sync, U: BevyRegistryKindErased<T> + TransparentWrapper<BoxAnyTypeMap>>(
    command_queue_tx: Option<BevyCommandQueueTx>,
    // mut signal_registry: U,
    mut signal_registry: WriteLock<
        '_,
        U,
        UnsyncStorage,
        SignalSubscriberDrop<ResourcesErased, UnsyncStorage>,
    >,
    request: U::RequestKind,
) -> SyncSignal<BevyValue<T>> {

    if let Some(command_queue_tx) = command_queue_tx {
        let mut commands = CommandQueue::default();

        let channels: DioxusTxrX<_> = request.clone().into();
        let command = request;

        let dioxus_rx = channels.dioxus_rx.clone();
        let dioxus_tx = channels.dioxus_tx.clone();
        commands.push(command);

        let new_signal = SyncSignal::new_maybe_sync(BevyValue {
            value: Err(BevyFetchBackup::Uninitialized),
            reader: Some(dioxus_rx),
            writer: Some(dioxus_tx),
        });
        let signal_registry = U::peel_mut(&mut signal_registry);
        signal_registry.insert(TypeId::of::<T>(), Box::new(new_signal.clone()));
        let _ = command_queue_tx.0
            .send(commands)
            .inspect_err(|err| warn!("{:#}", err));

        return new_signal;
    } else {
        let new_signal = SyncSignal::new_maybe_sync(BevyValue {
            value: Err(BevyFetchBackup::Unknown),
            reader: None,
            writer: None,
        });
        new_signal
    }


}