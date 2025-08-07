use std::{any::{type_name, Any, TypeId}, collections::HashMap, marker::PhantomData, rc::Rc, sync::Arc};
use bevy_app::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use bevy_log::{tracing_subscriber::registry, warn};

use crate::dioxus_in_bevy_plugin::{DioxusTxRxChannelsUntyped, DioxusTxRxChannelsUntypedRegistry};

// use crate::dioxus_in_bevy_plugin::DioxusChannelsUntypedRegistry;

// use crate::dioxus_in_bevy_plugin::{DioxusRxChannelsUntypedRegistry, DioxusTxChannelsUntypedRegistry};


pub mod dioxus_in_bevy_plugin;
pub mod ui;
pub mod traits;
mod systems;

/// A more restricted anymap for storing erased generics with sub generics, and indexing them via their sub-generic. 
pub trait ErasedSubGenericMap
    where
        Self: TransparentWrapper<AnytypeMap> + Sized,
{
    type Generic<T: Send + Sync + 'static>: Send + Sync + 'static;
    fn insert<T: Send + Sync + 'static>(&mut self, value: Self::Generic<T>)
        where
            // Self::Generic<T>: From<T>,
    {   
        let map = TransparentWrapper::peel_mut(self);
        let erased: Arc<dyn Any + Send + Sync> = Arc::new(value);
        map.insert(TypeId::of::<T>(), erased);
    }

    fn get<T: Send + Sync + 'static>(&mut self) -> Option<Arc<Self::Generic<T>>> {
        let map = TransparentWrapper::peel_mut(self);

        let value = map.get(&TypeId::of::<T>())?.clone();
        value.downcast::<Self::Generic<T>>().inspect_err(|err| warn!("could not downcast: {:#}", type_name::<T>())).ok()
    }
    fn extend(&mut self, value: Self) {
        let map = TransparentWrapper::peel_mut(self);
        let value = TransparentWrapper::peel(value);
        map.extend(value);
    }
}

pub struct SenderReceiver<T: Send + Sync + 'static> {
    pub sender: Sender<T>,
    pub receiver: Receiver<T>
}

/// An untyped hashmap that resolved typed entries by their type id.
pub type AnytypeMap = HashMap<TypeId, Arc<dyn Any + Send + Sync>>;

#[derive(Clone, Default, Debug, TransparentWrapper)]
#[repr(transparent)]
pub struct TxChannelRegistry(AnytypeMap);

impl ErasedSubGenericMap for TxChannelRegistry {
    type Generic<T: Send + Sync + 'static> = Sender::<T>;
}

#[derive(Clone, Default, Debug, TransparentWrapper)]
#[repr(transparent)]
pub struct RxChannelRegistry(AnytypeMap);

impl ErasedSubGenericMap for RxChannelRegistry {
    type Generic<T: Send + Sync + 'static> = Receiver::<T>;
}

pub struct BevyRxChannelTypeId(TypeId);


#[derive(Clone, Resource, Default)]
pub struct BevyTxChannelChannelsUntyped(pub TxChannelRegistry);

#[derive(Clone, Resource, Default)]
pub struct BevyRxChannelChannelsUntyped(pub RxChannelRegistry);


#[derive(Resource, Clone, Default, Debug)]
pub struct DioxusTxChannelsUntyped(pub TxChannelRegistry);

#[derive(Resource, Clone, Default, Debug)]
pub struct DioxusRxChannelsUntyped(pub RxChannelRegistry);


pub struct UiMessageRegistration<T: Send + Sync + 'static> {
    _a: PhantomData<T>
}

impl<T: Send + Sync + 'static> Default for UiMessageRegistration<T> {
    fn default() -> Self {
        Self { _a: Default::default() }
    }
}

/// Bevy side channel for giving [`T`] to dioxus
#[derive(Resource)]
pub struct BevyTxChannel<T>(pub Sender<T>);

/// Dioxus side channel for receiving [`T`] from bevy. 
#[derive(Resource)]
pub struct BevyRxChannel<T>(pub Receiver<T>);

/// Dioxus side channel for sending [`T`] to bevy
pub struct DioxusTxChannel<T>(pub Sender<T>);

/// Bevy side channel for receiving [`T`] from dioxus. 
#[derive(Resource)]
pub struct DioxusRxChannel<T>(pub Receiver<T>);

impl<M: Send + Sync + 'static> Plugin for UiMessageRegistration<M> {
    fn build(&self, app: &mut App) {
    
        let (bevy_tx, dioxus_rx) = crossbeam_channel::unbounded::<M>();
        let (dioxus_tx, bevy_rx) = crossbeam_channel::unbounded::<M>();

        let mut bevy_rx_channels = app.world_mut().get_resource_or_init::<BevyRxChannelChannelsUntyped>();

        bevy_rx_channels.0.insert::<M>(bevy_rx.clone());


        let mut bevy_tx_channels = app.world_mut().get_resource_or_init::<BevyTxChannelChannelsUntyped>();

        bevy_tx_channels.0.insert::<M>(bevy_tx.clone());


        app
        .insert_resource(BevyTxChannel(bevy_tx))
        .insert_resource(BevyRxChannel(bevy_rx))
        ;

        let dioxus_tx_channels = {
            let mut channels = app.world_mut().get_resource_or_init::<DioxusTxChannelsUntyped>();
            channels.0.insert(dioxus_tx);
            channels.clone()
        };

        let dioxus_rx_channels = {
            let mut channels = app.world_mut().get_resource_or_init::<DioxusRxChannelsUntyped>();
            channels.0.insert(dioxus_rx);
            channels.clone()
        };

        let dioxus_txrx_channels = DioxusTxRxChannelsUntyped {
            tx: dioxus_tx_channels,
            rx: dioxus_rx_channels
        };

        let dioxus_channels_registry = app.world_mut().get_resource_mut::<DioxusTxRxChannelsUntypedRegistry>().unwrap();

        dioxus_channels_registry.txrx.send(dioxus_txrx_channels);
    }
}


// #[derive(Resource)]
// pub struct DioxusRxChannelObtained<T>(bool, PhantomData<T>);