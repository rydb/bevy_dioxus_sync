use std::{
    any::{TypeId, type_name},
    collections::HashMap,
    fmt::Debug,
};

use bevy_app::prelude::*;
use bevy_dioxus_interop::{
    InfoPacket, add_systems_through_world,
    traits::{CrossDomSignalErasedMap, ErasedSignal, ErasedSignalValue},
};
use bevy_ecs::prelude::*;
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use dioxus_signals::Signal;

// #[derive(TransparentWrapper, Clone)]
// #[repr(transparent)]
// pub struct RequestBevyResource<T: Resource + Clone>(
//     // pub(crate) BevyDioxusIO<ResourceValue<T>, ResourceInfoIndex, ResourceAdditionalInfo>,
//     pub(crate) BevyDioxusPacket<ResourceValue<T>, ResourceInfoIndex, ResourceAdditionalInfo>,
// );

#[derive(TransparentWrapper, Clone)]
#[repr(transparent)]
pub struct RequestBevyResource<T: Resource + Clone>(
    // pub(crate) BevyDioxusIO<ResourceValue<T>, ResourceInfoIndex, ResourceAdditionalInfo>,
    pub(crate) ErasedSignalValue<T>,
);

// lay out types like this to prevent de-sync between systems and backend logistics updates.
pub type ResourceInfoIndex = TypeId;
pub type ResourceValue<T> = T;
pub type ResourceAdditionalInfo = ();
pub type ResourceInfoPacket<T> =
    InfoPacket<ResourceValue<T>, ResourceInfoIndex, ResourceAdditionalInfo>;

impl<T: Resource + Clone> Default for RequestBevyResource<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[derive(TransparentWrapper, Clone, Default)]
#[repr(transparent)]
pub struct BevyResourcesSignalsCache(Signal<BevyResourceSignals>);

/// type erased map of bevy resources
#[derive(TransparentWrapper, Resource, Default)]
#[repr(transparent)]
pub struct BevyResourceSignals(HashMap<TypeId, ErasedSignal>);

impl CrossDomSignalErasedMap for BevyResourceSignals {
    type Index = TypeId;

    type AdditionalInfo = ();
}

impl<T: Resource + Clone + Debug> Command for RequestBevyResource<T> {
    fn apply(self, world: &mut World) -> () {
        world.init_resource::<BevyResourceSignals>();

        let resource = world.get_resource::<T>().unwrap().clone();

        // self.0.initialize(ptr);
        let mut bevy_resource_signals = world.get_resource_mut::<BevyResourceSignals>().unwrap();

        //initialize or point signal to currently registered signal
        match bevy_resource_signals.get_typed::<T>(&TypeId::of::<T>()) {
            Some(n) => {
                self.0.pnt_to(n.get_ptr().unwrap());
            }
            None => {
                self.0.initialize(resource);
                bevy_resource_signals.insert_signal(self.0, TypeId::of::<T>());
            }
        };

        add_systems_through_world(
            world,
            Update,
            send_resource_update::<T>.run_if(resource_changed::<T>),
        );
    }
}

fn send_resource_update<T: Resource + Clone + Debug>(
    resource: Res<T>,
    mut bevy_resource_signal: ResMut<BevyResourceSignals>,
) {
    match bevy_resource_signal.get_typed::<T>(&TypeId::of::<T>()) {
        Some(n) => {
            let _set_status = n
                .set(resource.clone())
                .inspect_err(|err| warn!("error for {}: {}", err, type_name::<T>()));
        }
        None => {
            bevy_resource_signal.insert_value(resource.clone(), TypeId::of::<T>());
        }
    }
}
