use std::any::TypeId;

use bevy_ecs::prelude::*;
use bytemuck::TransparentWrapper;
use dioxus_signals::{Signal, SyncSignal};
use std::fmt::Debug;

use crate::{
    BevyValue, BoxGenericTypeMap, SignalsErasedMap, resource::command::RequestBevyResource,
    use_bevy_value,
};

/// Dioxus signals of Dioxus copies of Bevy resources.
#[derive(Clone, Default, TransparentWrapper)]
#[repr(transparent)]
pub struct ResourceRegistry(Signal<BevyResources>);

/// type erased map of bevy resources
#[derive(TransparentWrapper, Default)]
#[repr(transparent)]
pub struct BevyResources(BoxGenericTypeMap<TypeId>);

impl SignalsErasedMap for BevyResources {
    type Index = TypeId;

    type AdditionalInfo = ();
}

/// hook to interface with a bevy resource
pub fn use_bevy_resource<T: Debug + Resource + Send + Sync + Clone>()
-> SyncSignal<BevyValue<T, TypeId, ()>> {
    use_bevy_value::<T, ResourceRegistry, BevyResources, RequestBevyResource<T>>(Some(
        TypeId::of::<T>(),
    ))
}
