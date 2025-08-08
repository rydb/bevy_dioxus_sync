use std::{any::{type_name, Any, TypeId}, collections::HashMap, marker::PhantomData, rc::Rc, sync::Arc};
use bevy_app::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::{ComponentHooks, Immutable, StorageType}, prelude::*};
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use bevy_log::{tracing_subscriber::registry, warn};

use crate::{dioxus_in_bevy_plugin::{DioxusTxRxChannelsUntyped, DioxusTxRxChannelsUntypedRegistry}, systems::{DioxusPanelUpdates, PanelUpdate, PanelUpdateKind}, traits::DioxusElementMarker};

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



/// Bevy side channel for giving [`T`] to dioxus
#[derive(Resource)]
struct BevyTxChannel<T>(pub Sender<T>);

/// Dioxus side channel for receiving [`T`] from bevy. 
#[derive(Resource)]
struct BevyRxChannel<T>(pub Receiver<T>);

/// Dioxus side channel for sending [`T`] to bevy
pub struct DioxusTxChannel<T>(pub Sender<T>);

/// Bevy side channel for receiving [`T`] from dioxus. 
#[derive(Resource)]
pub struct DioxusRxChannel<T>(pub Receiver<T>);

// pub struct UiMessageRegistration<T: Send + Sync + 'static> {
//     _a: PhantomData<T>
// }

// impl<T: Send + Sync + 'static> Default for UiMessageRegistration<T> {
//     fn default() -> Self {
//         Self { _a: Default::default() }
//     }
// }


// impl<M: Send + Sync + 'static> Plugin for UiMessageRegistration<M> {
//     fn build(&self, app: &mut App) {
    
//         let (bevy_tx, dioxus_rx) = crossbeam_channel::unbounded::<M>();
//         let (dioxus_tx, bevy_rx) = crossbeam_channel::unbounded::<M>();

//         let mut bevy_rx_channels = app.world_mut().get_resource_or_init::<BevyRxChannelChannelsUntyped>();

//         bevy_rx_channels.0.insert::<M>(bevy_rx.clone());


//         let mut bevy_tx_channels = app.world_mut().get_resource_or_init::<BevyTxChannelChannelsUntyped>();

//         bevy_tx_channels.0.insert::<M>(bevy_tx.clone());


//         app
//         .insert_resource(BevyTxChannel(bevy_tx))
//         .insert_resource(BevyRxChannel(bevy_rx))
//         ;

//         let dioxus_tx_channels = {
//             let mut channels = app.world_mut().get_resource_or_init::<DioxusTxChannelsUntyped>();
//             channels.0.insert(dioxus_tx);
//             channels.clone()
//         };

//         let dioxus_rx_channels = {
//             let mut channels = app.world_mut().get_resource_or_init::<DioxusRxChannelsUntyped>();
//             channels.0.insert(dioxus_rx);
//             channels.clone()
//         };

//         let dioxus_txrx_channels = DioxusTxRxChannelsUntyped {
//             tx: dioxus_tx_channels,
//             rx: dioxus_rx_channels
//         };

//         let dioxus_channels_registry = app.world_mut().get_resource_mut::<DioxusTxRxChannelsUntypedRegistry>().unwrap();

//         dioxus_channels_registry.txrx.send(dioxus_txrx_channels);
//     }
// }


/// Component that marks an entity as a dioxus panel
#[derive(Clone, Debug)]
pub struct DioxusPanel {
    pub(crate) element_marker: Arc<dyn DioxusElementMarker>
}

impl DioxusPanel {
    pub fn new<T: DioxusElementMarker>(element: T) -> Self {
        Self {
            element_marker: Arc::new(element)
        }
    }
}

impl Component for DioxusPanel {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    /// to change the panel on this entity, insert a new one.
    type Mutability = Immutable;

     fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, hook| {
            let Some(value) = world.entity(hook.entity).get::<Self>() else {
                warn!("could not get {:#} on {:#}", type_name::<Self>(), hook.entity);
                return
            };
            let value = value.clone();
            let mut panel_updates = world.get_resource_mut::<DioxusPanelUpdates>().unwrap();
            warn!("pushing panel update for {:#} to {:#?}", hook.entity, PanelUpdateKind::Add(value.clone()));
            panel_updates.0.push(PanelUpdate { key: hook.entity, value: PanelUpdateKind::Add(value) })
        });
        hooks.on_remove(|mut world, hook| {
            let mut panel_updates = world.get_resource_mut::<DioxusPanelUpdates>().unwrap();
            panel_updates.0.push(PanelUpdate { key: hook.entity, value: PanelUpdateKind::Remove })
        });
    }
}


/// bevy resource marked handle updates to/from dioxus.
pub struct UiResourceRegistration<T: Send + Sync + Clone + 'static + Resource> {
    _a: PhantomData<T>
}

impl<T: Send + Sync + Clone + Resource + 'static> Default for UiResourceRegistration<T> {
    fn default() -> Self {
        Self { _a: Default::default() }
    }
}


impl<M: Send + Sync + Clone + Resource + 'static> Plugin for UiResourceRegistration<M> {
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
        .add_systems(Update, send_resource_update::<M>.run_if(resource_changed::<M>))
        .add_systems(Update, receive_resource_update::<M>)
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


fn send_resource_update<T: Resource + Clone>(
    resource: Res<T>,
    bevy_tx: ResMut<BevyTxChannel<T>>,
    // bevy_rx: ResMut<BevyRxChannel<T>>,
) {
    bevy_tx.0.send(resource.clone());
}

fn receive_resource_update<T: Resource + Clone>(
    mut resource: ResMut<T>,
    bevy_rx: ResMut<BevyRxChannel<T>>,
    // bevy_rx: ResMut<BevyRxChannel<T>>,
) {
    let Ok(new_res) = bevy_rx.0.try_recv() else {
        return
    };
    *resource = new_res;
}