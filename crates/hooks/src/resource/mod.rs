pub mod command;
pub mod hook;

use std::{any::TypeId, collections::HashMap};

use bevy_dioxus_interop::{BevyRxChannel, BoxAnyTypeMap};
use bevy_ecs::prelude::*;
use bevy_app::prelude::*;
use bytemuck::TransparentWrapper;

use crate::resource::command::{BevyResourceSignals, ResourceInfoPacket};

// #[derive(TransparentWrapper ,Resource)]
// #[repr(transparent)]
// pub struct BevyResourceSignals(BoxGenericTypeMap<TypeId>);

// impl SignalsErasedMap for BevyResourceSignals {
//     type Index = TypeId;

//     type AdditionalInfo = ();
// }

// impl<T: Resource> Command for SyncResourceWithBevy<T> {
//     fn apply(self, world: &mut World) -> () {
//         Self {

//         }
//     }
// }

pub struct BevyResourcesSignalsPlugin;

impl Plugin for BevyResourcesSignalsPlugin {
    fn build(&self, app: &mut App) {
       app.init_resource::<BevyResourceSignals>()    
       ;
    }
}
