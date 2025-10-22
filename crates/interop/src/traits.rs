use std::{
    any::{Any, TypeId, type_name},
    sync::Arc,
};

use bevy_log::warn;
use bytemuck::TransparentWrapper;

use crate::ArcAnytypeMap;

pub trait ErasedSubGenericMap
where
    Self: TransparentWrapper<ArcAnytypeMap> + Sized,
{
    type Generic<T: Send + Sync + 'static>: Send + Sync + 'static;
    fn insert<T: Send + Sync + 'static>(&mut self, value: Self::Generic<T>) {
        let map = TransparentWrapper::peel_mut(self);
        let erased: Arc<dyn Any + Send + Sync> = Arc::new(value);
        map.insert(TypeId::of::<T>(), erased);
    }

    fn get<T: Send + Sync + 'static>(&mut self) -> Option<Arc<Self::Generic<T>>> {
        let map = TransparentWrapper::peel_mut(self);

        let value = map.get(&TypeId::of::<T>())?.clone();
        value
            .downcast::<Self::Generic<T>>()
            .inspect_err(|err| warn!("could not downcast: {:#}: {:#?}", type_name::<T>(), err))
            .ok()
    }
    fn extend(&mut self, value: Self) {
        let map = TransparentWrapper::peel_mut(self);
        let value = TransparentWrapper::peel(value);
        map.extend(value);
    }
}
