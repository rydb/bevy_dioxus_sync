use std::{any::{type_name, Any, TypeId}, collections::HashMap, marker::PhantomData, rc::Rc, sync::Arc};
use bevy_app::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use bevy_log::{tracing_subscriber::registry, warn};

use crate::dioxus_in_bevy_plugin::{DioxusRxChannelsUntypedRegistry, DioxusTxChannelsUntypedRegistry};


pub mod dioxus_in_bevy_plugin;

/// A more restricted anymap for storing erased generics with sub generics, and indexing them via their sub-generic. 
pub trait ErasedSubGenericMap
    where
        Self: TransparentWrapper<HashMap<TypeId, Arc<dyn Any + Send + Sync>>> + Sized,
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

// #[derive(Clone, Default, Debug, TransparentWrapper)]
// #[repr(transparent)]
// pub struct ChannelRegistry(HashMap<TypeId, Arc<dyn Any + Send + Sync>>);

#[derive(Clone, Default, Debug, TransparentWrapper)]
#[repr(transparent)]
pub struct TxChannelRegistry(HashMap<TypeId, Arc<dyn Any + Send + Sync>>);

impl ErasedSubGenericMap for TxChannelRegistry {
    type Generic<T: Send + Sync + 'static> = Sender::<T>;
}

#[derive(Clone, Default, Debug, TransparentWrapper)]
#[repr(transparent)]
pub struct RxChannelRegistry(HashMap<TypeId, Arc<dyn Any + Send + Sync>>);

impl ErasedSubGenericMap for RxChannelRegistry {
    type Generic<T: Send + Sync + 'static> = Receiver::<T>;
}

// impl ErasedSubGenericMap for ChannelRegistry {
//     type Generic<T: Send + Sync + 'static> = SenderReceiver::<T>;
// }

pub struct BevyRxChannelTypeId(TypeId);

// #[derive(Clone, Resource, Default, Debug)]
// /// A registry of Bevy state channels to be recieved by and interacted with through Dioxus.
// pub struct DioxusIOChannels(pub ChannelRegistry);

// /// A registry of Dioxus state channels to be recieved by and interacted with through Bevy.
// #[derive(Clone, Resource, Default, Debug)]
// pub struct BevyIOChannels(pub ChannelRegistry);


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
        {
            // let mut bevy_io_channels_registry = app.world_mut().get_resource_or_init::<BevyIOChannels>();


            let (bevy_tx, dioxus_rx) = crossbeam_channel::unbounded::<M>();
            let (dioxus_tx, bevy_rx) = crossbeam_channel::unbounded::<M>();

            
            // let bevy_io_channels = SenderReceiver {
            //     sender: bevy_sender.clone(),
            //     receiver: bevy_receiver.clone(),
            // };

            // let mut bevy_sender_channels = app.world_mut().get_resource_or_init::<BevyTxChannelChannelsUntyped>();

            // bevy_sender_channels.0.insert::<T>(bevy_sender.clone());
            

            // let mut bevy_sender_channels = bevy_sender_channels.clone();
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

            let dioxus_tx_registry = app.world_mut().get_resource_mut::<DioxusTxChannelsUntypedRegistry>().unwrap();
            dioxus_tx_registry.0.send(dioxus_tx_channels);

            let dioxus_rx_registry = app.world_mut().get_resource_mut::<DioxusRxChannelsUntypedRegistry>().unwrap();
            dioxus_rx_registry.0.send(dioxus_rx_channels);
            // bevy_io_channels_registry.0.insert::<T>(bevy_io_channels);
            
            // let bevy_receiver = BevyRxChannel(bevy_receiver);

            // let registry = bevy_io_channels_registry.clone();

            // let receiver_registry = bevy_receiver_channels.clone();
            // app
            // .insert_resource::<BevyTxChannel<M>>(BevyTxChannel(bevy_sender));

            // let registry_sender = app.world_mut().get_resource_mut::<BevyIOChannelsSender>().unwrap();
            
            // let sender_registry_sender = app.world_mut().get_resource_mut::<BevyTxChannelChannelsUntypedSender>().unwrap();
            // sender_registry_sender.0.send(sender_registry);

            // let receiver_registry_sender = app.world_mut().get_resource_mut::<BevyRxChannelChannelsUntypedSender>().unwrap();
            // receiver_registry_sender.0.send(receiver_registry);
            
            // let send_status = registry_sender.0.send(registry);
            // warn!("bevy io channels registry send status: {:#?}", send_status);
        }
        {



            // let mut dioxus_io_channels_registry = app.world_mut().get_resource_or_init::<DioxusIOChannels>(); 

            // let (dioxus_sender, dioxus_receiver) = crossbeam_channel::unbounded::<T>();

            // let dioxus_io_channels = SenderReceiver {
            //     sender: dioxus_sender.clone(),
            //     receiver: dioxus_receiver.clone(),
            // };
            // dioxus_io_channels_registry.0.insert::<T>(dioxus_io_channels);

            // let registry = dioxus_io_channels_registry.clone();

            // app
            // .insert_resource::<DioxusRxChannel<T>>(DioxusRxChannel(dioxus_receiver))
            // ;

            // let registry_sender = app.world_mut().get_resource_mut::<DioxusIOChanelsSender>().unwrap();
            // let send_status = registry_sender.0.send(registry);
            // warn!("dioxus io channels registry send status: {:#?}", send_status);
        }

        // app.add_systems(Update, poll_for_dioxus_receiver::<T>);

    }
}


#[derive(Resource)]
pub struct DioxusRxChannelObtained<T>(bool, PhantomData<T>);

// #[derive(Resource, Default)]
// pub struct DioxusStateReceivers(ChannelRegistry);

// #[derive(Resource, Default)]
// pub struct DioxusStateSenders(RecieverChannels);

// pub fn poll_for_dioxus_receiver<T: Send + Sync + 'static>(
//     mut dioxus_receiver: ResMut<DioxusRxChannel<T>>,
//     mut dioxus_receivers_registry: ResMut<DioxusStateReceivers>,
// ) {
//     let Some(receiver) = dioxus_receivers_registry.0.get::<T>() else {
//         return
//     };
//     dioxus_receiver.0 = Some(receiver.clone())
// }

// impl ChannelRegistry {
//     pub fn new() -> Self {
//         ChannelRegistry(HashMap::new())
//     }

//     pub fn register<T: Send + Sync + 'static>(&mut self) -> SenderReceiver<T> {
        
//         let (sender, receiver) = crossbeam_channel::unbounded::<T>();

//         let sender_receiver = SenderReceiver {
//             sender: sender.clone(),
//             receiver: receiver.clone()
//         };
//         let erased: Arc<dyn Any + Send + Sync> = Arc::new(SenderReceiver {
//             sender: sender,
//             receiver,
//         });
//         self.0.insert(TypeId::of::<T>(), erased);
//         sender_receiver
//     }

//     pub fn get<T: Send + Sync + 'static>(&mut self) -> Option<Arc<SenderReceiver<T>>> {
//         let value = self.0.get(&TypeId::of::<T>())?.clone();
//         value.downcast::<SenderReceiver<T>>().inspect_err(|err| warn!("could not downcast: {:#}", type_name::<T>())).ok()
//     }
//     pub fn extend(&mut self, value: Self) {
//         self.0.extend(value.0);
//     }
// }