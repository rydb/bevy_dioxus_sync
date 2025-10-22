use std::any::TypeId;

use bevy_ecs::prelude::*;
use bytemuck::TransparentWrapper;
use dioxus_signals::{Signal, SyncSignal};

use crate::{
    BevyValue, BoxGenericTypeMap, SignalsErasedMap, resource::command::RequestBevyResource,
    use_bevy_value,
};

/// Dioxus signals of Dioxus copies of Bevy resources.
#[derive(Clone, Default, TransparentWrapper)]
#[repr(transparent)]
pub struct ResourceRegistry(Signal<BevyResources>);

// #[derive(TransparentWrapper, Default)]
// #[repr(transparent)]
// pub struct BevyResources(BoxGenericTypeMap<TypeId>);

#[derive(TransparentWrapper, Default)]
#[repr(transparent)]
pub struct BevyResources(BoxGenericTypeMap<TypeId>);

// impl<T: Resource + Clone> BevyRegistryKindErased<T> for BevyResources {
//     type RequestKind = RequestBevyResource<T>;
// }

// impl SignalsErasedMap for BevyResources {
//     type Value<T: Clone + Send + Sync + 'static> = SyncSignal<BevyValue<T, TypeId>>;

//     type Index = TypeId;
// }

impl SignalsErasedMap for BevyResources {
    type Index = TypeId;

    type AdditionalInfo = ();
}

/// requests a resource from bevy.
pub fn use_bevy_resource<T: Resource + Send + Sync + Clone>() -> SyncSignal<BevyValue<T, TypeId, ()>>
{
    use_bevy_value::<T, ResourceRegistry, BevyResources, RequestBevyResource<T>>(Some(
        TypeId::of::<T>(),
    ))
}
