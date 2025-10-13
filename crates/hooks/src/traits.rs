//! TODO: Find better alternative to copying one trait bound across each generic map.
//!
//! can't get assocaited types to have "stricter impl's" then their oriignal definition, so this
//! is what we do to side step that...

use bevy_asset::{Asset, Handle};
use bevy_dioxus_interop::BoxAnyTypeMap;
use bevy_ecs::{component::Component, resource::Resource};
use bytemuck::TransparentWrapper;
use dioxus_signals::SyncSignal;
use std::{
    any::TypeId, ops::Deref, 
};

pub trait ErasedSubGenericResourcecMap
where
    Self: TransparentWrapper<BoxAnyTypeMap> + Sized,
{
    type Generic<T: Clone + Resource + Send + Sync>: Send + Sync + Clone + 'static;
    fn insert<T: Clone + Resource + Send + Sync + 'static>(&mut self, value: Self::Generic<T>) {
        let map = TransparentWrapper::peel_mut(self);
        let erased = Box::new(value);
        map.insert(TypeId::of::<T>(), erased);
    }

    fn get<T: Clone + Resource + Send + Sync + 'static>(
        &mut self,
    ) -> Option<&mut Self::Generic<T>> {
        let map = TransparentWrapper::peel_mut(self);

        let value = map.get_mut(&TypeId::of::<T>())?;

        value.downcast_mut::<Self::Generic<T>>()
    }
    fn extend(&mut self, value: Self) {
        let map = TransparentWrapper::peel_mut(self);
        let value = TransparentWrapper::peel(value);
        map.extend(value);
    }
}

pub trait ErasedSubGenericComponentsMap
where
    Self: TransparentWrapper<BoxAnyTypeMap> + Sized,
{
    type Generic<T: Clone + Component + Send + Sync>: Send + Sync + Clone + 'static;
    fn insert<T: Clone + Component + Send + Sync + 'static>(&mut self, value: Self::Generic<T>) {
        let map = TransparentWrapper::peel_mut(self);
        let erased = Box::new(value);
        map.insert(TypeId::of::<T>(), erased);
    }

    fn get<T: Clone + Component + Send + Sync + 'static>(
        &mut self,
    ) -> Option<&mut Self::Generic<T>> {
        let map = TransparentWrapper::peel_mut(self);

        let value = map.get_mut(&TypeId::of::<T>())?;

        value.downcast_mut::<Self::Generic<T>>()
    }
    fn extend(&mut self, value: Self) {
        let map = TransparentWrapper::peel_mut(self);
        let value = TransparentWrapper::peel(value);
        map.extend(value);
    }
}

pub trait ErasedSubGenericComponentSingletonMap
where
    Self: TransparentWrapper<BoxAnyTypeMap> + Sized,
{
    type Generic<T: Component + Clone, U: Component>: Send + Sync + Clone + 'static;
    fn insert<T, U>(&mut self, value: Self::Generic<T, U>)
    where
        T: Component + Clone + Send + Sync + 'static,
        U: Component,
    {
        let map = TransparentWrapper::peel_mut(self);
        let erased = Box::new(value);
        map.insert(TypeId::of::<T>(), erased);
    }

    fn get<T, U>(&mut self) -> Option<&mut Self::Generic<T, U>>
    where
        T: Component + Clone + Send + Sync + 'static,
        U: Component,
    {
        let map = TransparentWrapper::peel_mut(self);

        let value = map.get_mut(&TypeId::of::<T>())?;

        value.downcast_mut::<Self::Generic<T, U>>()
    }
    fn extend(&mut self, value: Self) {
        let map = TransparentWrapper::peel_mut(self);
        let value = TransparentWrapper::peel(value);
        map.extend(value);
    }
}

pub trait ErasedSubGenericAssetsMap
where
    Self: TransparentWrapper<BoxAnyTypeMap> + Sized,
{
    type Generic<T: Deref<Target = Handle<U>> + Component, U: Asset + Clone>: Send
        + Sync
        + Clone
        + 'static;
    fn insert<T, U>(&mut self, value: Self::Generic<T, U>)
    where
        T: Deref<Target = Handle<U>> + Component + 'static,
        U: Asset + Clone + Send + Sync + 'static,
    {
        let map = TransparentWrapper::peel_mut(self);
        let erased = Box::new(value);
        map.insert(TypeId::of::<T>(), erased);
    }

    fn get<T, U>(&mut self) -> Option<&mut Self::Generic<T, U>>
    where
        T: Deref<Target = Handle<U>> + Component + 'static,
        U: Asset + Clone + Send + Sync + 'static,
    {
        let map = TransparentWrapper::peel_mut(self);

        let value = map.get_mut(&TypeId::of::<T>())?;

        value.downcast_mut::<Self::Generic<T, U>>()
    }
    fn extend(&mut self, value: Self) {
        let map = TransparentWrapper::peel_mut(self);
        let value = TransparentWrapper::peel(value);
        map.extend(value);
    }
}


// pub trait BevySignalSource 
//     type Signal;
//     type SignalHolder;
// {
//     fn request_signal() -> SyncSignal<>
// }