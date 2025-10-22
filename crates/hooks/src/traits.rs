use crate::{BoxGenericTypeMap, SignalErasedMapValue};
use bytemuck::TransparentWrapper;
use std::hash::Hash;

pub trait SignalsErasedMap
where
    Self: TransparentWrapper<BoxGenericTypeMap<Self::Index>> + Sized,
{
    // type Value: Clone + 'static + Send + Sync;
    type Index: Hash + Eq + Clone + Send + Sync + 'static;
    type AdditionalInfo: Send + Sync + 'static;
    fn insert_typed<T: Clone + Send + Sync + 'static>(
        &mut self,
        value: SignalErasedMapValue<T, Self::Index, Self::AdditionalInfo>,
        index: Self::Index,
    ) {
        let map = TransparentWrapper::peel_mut(self);
        let erased = Box::new(value);
        map.insert(index, erased);
    }

    fn get_typed<T: Clone + Send + Sync + 'static>(
        &mut self,
        index: &Self::Index,
    ) -> Option<&mut SignalErasedMapValue<T, Self::Index, Self::AdditionalInfo>> {
        let map = TransparentWrapper::peel_mut(self);

        let value = map.get_mut(&index)?;

        value.downcast_mut::<SignalErasedMapValue<T, Self::Index, Self::AdditionalInfo>>()
    }
}
