use std::fmt::Debug;

use bevy_app::prelude::*;
use bevy_dioxus_interop::{
    add_systems_through_world,
    signals::CrossDomSignal,
};
use bevy_ecs::prelude::*;
use bevy_log::warn;
use bytemuck::TransparentWrapper;

// #[derive(TransparentWrapper, Clone, Default)]
// #[repr(transparent)]
// pub struct BevyResourcesSignalsCache(Signal<BevyResourceSignals>);

// /// type erased map of bevy resources
// #[derive(TransparentWrapper, Resource, Default)]
// #[repr(transparent)]
// pub struct BevyResourceSignals(HashMap<TypeId, ErasedSignal>);

// impl CrossDomSignalErasedMap for BevyResourceSignals {
//     type Index = TypeId;

//     type AdditionalInfo = ();
// }

#[derive(TransparentWrapper, Resource, Clone)]
#[repr(transparent)]
pub struct BevyResourceClone<T>(CrossDomSignal<T>);

#[derive(TransparentWrapper, Clone)]
#[repr(transparent)]
pub struct RequestBevyResource<T: Resource + Clone>(pub(crate) BevyResourceClone<T>);

impl<T: Resource + Clone + Debug> Command for RequestBevyResource<T> {
    fn apply(self, world: &mut World) -> () {
        let resource = world.get_resource::<T>().unwrap().clone();

        match world.get_resource::<BevyResourceClone<T>>() {
            Some(n) => self.0.0.pnt_to(n.0.get_ptr().unwrap()),
            None => {
                self.0.0.initialize(resource);
                world.insert_resource(self.0.clone());
            }
        }

        add_systems_through_world(
            world,
            Update,
            send_resource_update::<T>.run_if(resource_changed::<T>),
        );
    }
}

fn send_resource_update<T: Resource + Clone + Debug>(
    resource: Res<T>,
    bevy_resource_signal: Res<BevyResourceClone<T>>,
) {
    let _ = bevy_resource_signal
        .0
        .set(resource.clone())
        .inspect_err(|err| warn!("{err}"));
    // match bevy_resource_signal.get_typed::<T>(&TypeId::of::<T>()) {
    //     Some(n) => {
    //         let _set_status = n
    //             .set(resource.clone())
    //             .inspect_err(|err| warn!("error for {}: {}", err, type_name::<T>()));
    //     }
    //     None => {
    //         bevy_resource_signal.insert_value(resource.clone(), TypeId::of::<T>());
    //     }
    // }
}
