use std::any::TypeId;

use bevy_ecs::{component::Mutable, prelude::*};
use bytemuck::TransparentWrapper;
use dioxus_signals::{Signal, SyncSignal};
use std::fmt::Debug;

use crate::{
    BevyValue, BoxGenericTypeMap, SignalsErasedMap,
    component::component_single::command::RequestBevyComponentSingleton, use_bevy_value,
};

/// hook to interface with a singular bevy component, [`T`], with a marker [`U`]
pub fn use_bevy_component_singleton<T, U>() -> SyncSignal<BevyValue<T, TypeId, ()>>
where
    T: Debug + Component<Mutability = Mutable> + Clone,
    U: Component + Clone,
{
    use_bevy_value::<T, BevyComponentsRegistry, BevyComponents, RequestBevyComponentSingleton<T, U>>(
        None,
    )
}

#[derive(TransparentWrapper, Default)]
#[repr(transparent)]
pub struct BevyComponents(BoxGenericTypeMap<TypeId>);

impl SignalsErasedMap for BevyComponents {
    type Index = TypeId;

    type AdditionalInfo = ();
}

#[derive(Clone, TransparentWrapper, Default)]
#[repr(transparent)]
pub struct BevyComponentsRegistry(Signal<BevyComponents>);
