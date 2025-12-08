use std::{any::{TypeId, type_name}, collections::HashMap, fmt::{Debug, Display}};

use bevy_app::prelude::*;
use bevy_dioxus_interop::{
    BevyDioxusIO, BevyDioxusPacket, BevyRxChannel, BoxAnyTypeMap, InfoPacket, InfoUpdate, StatusUpdate, add_systems_through_world, signals::CrossDomSignal, traits::{CrossDomSignalErasedMap, ErasedSignal, ErasedSignalValue}
};
use bevy_ecs::prelude::*;
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use dioxus_core::ReactiveContext;
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
        // let x = Channels::<Self>::default();
        // world.insert_resource(BevyTxChannel(self.0.bevy_tx));
        // world.insert_resource(BevyRxChannel(self.0.io.bevy_rx));
        world.init_resource::<BevyResourceSignals>();

        let resource = world.get_resource::<T>().unwrap().clone();

        // self.0.initialize(ptr);
        let mut bevy_resource_signals = world.get_resource_mut::<BevyResourceSignals>().unwrap();

        //initialize or point signal to currently registered signal
        match bevy_resource_signals.get_typed::<T>(&TypeId::of::<T>()) {
            Some(n) => {
                self.0.pnt_to(n.get_ptr().unwrap());
            },
            None => {
                self.0.initialize(resource);
                bevy_resource_signals.insert_signal(self.0, TypeId::of::<T>());
                
            },
        };
        // println!("current ptr: {:#?}", current_ptr);
        // let current_ptr = match current_ptr {
        //     Some(ptr) => ptr,
        //     None => todo!(),
        // };

        // // //Initialize the requesting dioxus context's pointer so that it points to the main holder(the one bevy owns)
        // self.0.initialize(current_ptr);

        add_systems_through_world(
            world,
            Update,
            send_resource_update::<T>.run_if(resource_changed::<T>),
        );
        // add_systems_through_world(world, Update, receive_resource_update::<T>);
    }
}

fn send_resource_update<T: Resource + Clone + Debug>(
    resource: Res<T>,
    mut bevy_resource_signal: ResMut<BevyResourceSignals>
) {
    match bevy_resource_signal.get_typed::<T>(&TypeId::of::<T>()) {
        Some(n) => {

            let set_status = n.set(resource.clone());

            // warn!("set status: {:#?}, value: {:#?}", set_status, resource.clone());
        },
        None => {
            bevy_resource_signal.insert_value(resource.clone(), TypeId::of::<T>());

            // warn!("previous resource state not found for: {}", type_name::<T>());
        },
    }
}

// fn receive_resource_update<T: Resource + Clone>(
//     mut resource: ResMut<T>,
//     bevy_rx: ResMut<BevyRxChannel<ResourceInfoPacket<T>>>,
//     mut bevy_resource_signals: ResMut<BevyResourceSignals>

//     // bevy_tx: ResMut<BevyTxChannel<ResourceInfoPacket<T>>>,
// ) {
//     while let Ok(packet) = bevy_rx.0.try_recv().inspect_err(|err| match err {
//         crossbeam_channel::TryRecvError::Empty => {}
//         crossbeam_channel::TryRecvError::Disconnected => {
//             warn!("could not receive as channel is disconnected")
//         }
//     }) {
//         match packet {
//             InfoPacket::Update(info_update) => {
//                 match bevy_resource_signals.get_typed::<T>(&TypeId::of::<T>()) {
//                     Some(n) => {
//                         let _ = n.set(info_update.update.clone());
//                     },
//                     None => {
//                         warn!("Attemtped to update resource value before initial clone recieved. Skipping");
//                     },
//                 }
//                 *resource = info_update.update;
//             }
//             InfoPacket::Request(status_update) => match status_update {
//                 StatusUpdate::RequestRefresh => {
//                     todo!()
//                     //send_resource_update(resource.into(), bevy_tx)
//                 },
//             },
//         }
//         return;
//     }
// }
