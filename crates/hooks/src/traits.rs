use crate::{BoxGenericTypeMap, SignalErasedMapValue};
use bytemuck::TransparentWrapper;
use std::fmt::Debug;
use std::hash::Hash;

pub trait SignalsErasedMap
where
    Self: TransparentWrapper<BoxGenericTypeMap<Self::Index>> + Sized,
{
    /// What type a signal is indexed by
    type Index: Debug + Hash + Eq + Clone + Send + Sync + 'static;

    /// additional to be sent to/from bevy.
    type AdditionalInfo: Send + Sync + 'static + Clone;
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
