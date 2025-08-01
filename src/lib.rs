use std::{any::{type_name, Any, TypeId}, collections::HashMap, marker::PhantomData, rc::Rc, sync::Arc};
use bevy_app::prelude::*;
use bevy_ecs::resource::Resource;
use crossbeam_channel::{Receiver, Sender};
use bevy_log::warn;

use crate::dioxus_in_bevy_plugin::UiMessageRegistrySender;

pub mod dioxus_in_bevy_plugin;

pub struct SenderReceiver<T> {
    pub sender: Sender<T>,
    pub receiver: Receiver<T>
}


#[derive(Clone, Resource, Default, Debug)]
pub struct UiMessageRegistry(HashMap<TypeId, Arc<dyn Any + Send + Sync>>);


pub struct UiMessageRegistration<T: Send + Sync + 'static> {
    _a: PhantomData<T>
}

impl<T: Send + Sync + 'static> Default for UiMessageRegistration<T> {
    fn default() -> Self {
        Self { _a: Default::default() }
    }
}

#[derive(Resource)]
pub struct BevySender<T>(pub Sender<T>);

#[derive(Resource)]
pub struct BevyReceiver<T>(pub Receiver<T>);

impl<T: Send + Sync + 'static> Plugin for UiMessageRegistration<T> {
    fn build(&self, app: &mut App) {

        let mut registry = app.world_mut().get_resource_or_init::<UiMessageRegistry>();

        let sender_receiver = registry.register::<T>();
        
        let registry = registry.clone();
        app
        .insert_resource::<BevySender<T>>(BevySender(sender_receiver.sender))
        .insert_resource::<BevyReceiver<T>>(BevyReceiver(sender_receiver.receiver))
        ;
        let registry_sender = app.world_mut().get_resource_mut::<UiMessageRegistrySender>().unwrap();

        let send_status = registry_sender.0.send(registry);

        warn!("registry send status: {:#?}", send_status);
    }
}

impl UiMessageRegistry {
    pub fn new() -> Self {
        UiMessageRegistry(HashMap::new())
    }

    pub fn register<T: Send + Sync + 'static>(&mut self) -> SenderReceiver<T> {
        
        let (sender, receiver) = crossbeam_channel::unbounded::<T>();

        let sender_receiver = SenderReceiver {
            sender: sender.clone(),
            receiver: receiver.clone()
        };
        let erased: Arc<dyn Any + Send + Sync> = Arc::new(SenderReceiver {
            sender: sender,
            receiver,
        });
        self.0.insert(TypeId::of::<T>(), erased);
        sender_receiver
    }

    pub fn get<T: Send + Sync + 'static>(&mut self) -> Option<Arc<SenderReceiver<T>>> {
        let value = self.0.get(&TypeId::of::<T>())?.clone();
        value.downcast::<SenderReceiver<T>>().inspect_err(|err| warn!("could not downcast: {:#}", type_name::<T>())).ok()
    }
    pub fn extend(&mut self, value: Self) {
        self.0.extend(value.0);
    }
}