
use bevy_dioxus_interop::signals::CrossDomSignal;
use bevy_ecs::prelude::*;
use std::fmt::Debug;

use crate::{
    resource::command::{BevyResourceClone, RequestBevyResource},
    use_bevy_value,
};

/// hook to interface with a bevy resource
pub fn use_bevy_resource<T: Debug + Resource + Send + Sync + Clone>() -> CrossDomSignal<T> {
    use_bevy_value::<T, BevyResourceClone<T>, RequestBevyResource<T>, T>()
}
