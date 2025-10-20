use std::{any::{type_name, TypeId}, fmt::Display};

use async_std::task::sleep;
use bevy_dioxus_interop::{BevyCommandQueueTx, BoxAnyTypeMap, InfoRefershRateMS};
use bevy_ecs::{prelude::*, world::CommandQueue};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use dioxus_core::use_hook;
use dioxus_hooks::{try_use_context, use_context, use_future};
use dioxus_signals::{ReadableExt, Signal, SignalSubscriberDrop, SyncSignal, SyncStorage, UnsyncStorage, WritableExt, WriteLock};

use crate::{request_bevy_signal, resource::command::RequestBevyResource, use_bevy_value, BevyFetchBackup, BevyValue, BoxGenericTypeMap, SignalsErasedMap};

/// Dioxus signals of Dioxus copies of Bevy resources.
#[derive(Clone, Default, TransparentWrapper)]
#[repr(transparent)]
pub struct ResourceRegistry(Signal<BevyResources>);

#[derive(TransparentWrapper, Default)]
#[repr(transparent)]
pub struct BevyResources(BoxGenericTypeMap<TypeId>);

// impl<T: Resource + Clone> BevyRegistryKindErased<T> for BevyResources {
//     type RequestKind = RequestBevyResource<T>;
// } 

impl SignalsErasedMap for BevyResources {
    type Value<T: Clone + Send + Sync + 'static> = SyncSignal<BevyValue<T, TypeId>>;

    type Index = TypeId;
}

/// requests a resource from bevy.
pub fn use_bevy_resource<T: Resource + Send + Sync + Clone>() -> SyncSignal<BevyValue<T, TypeId>> {
    use_bevy_value::<T, ResourceRegistry, BevyResources, RequestBevyResource<T>>(Some(TypeId::of::<T>()))
}