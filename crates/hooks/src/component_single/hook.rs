use std::{any::{type_name, TypeId}, marker::PhantomData};

use async_std::task::sleep;
use bevy_dioxus_interop::{BevyCommandQueueTx, BoxAnyTypeMap, InfoRefershRateMS};
use bevy_ecs::{component::Mutable, prelude::*, world::CommandQueue};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use dioxus_core::use_hook;
use dioxus_hooks::{try_use_context, use_context, use_future};
use dioxus_signals::{Signal, SignalSubscriberDrop, SyncSignal, UnsyncStorage, WritableExt, WriteLock};

use crate::{component_single::{command::RequestBevyComponentSingleton, BevyComponentSingleton}, resource::hook::BevyResources, traits::ErasedSubGenericComponentSingletonMap, use_bevy_value, BevyValue, BoxGenericTypeMap, SignalsErasedMap};


pub fn use_bevy_component_singleton<T, U>() -> SyncSignal<BevyValue<T, TypeId, ()>>
where
    T: Component<Mutability = Mutable> + Clone,
    U: Component + Clone,
{
    use_bevy_value::<T, BevyComponentsRegistry, BevyComponents, RequestBevyComponentSingleton<T, U>>(None)
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