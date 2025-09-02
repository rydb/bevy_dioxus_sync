use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;
use crossbeam_channel::{Receiver, Sender};

pub mod command;
pub mod hook;

pub type EntityComponentQueue<T> = AddRemoveQueues<Entity, T>;
pub struct AddRemoveQueues<T, U> {
    pub add: HashMap<T, U>,
    pub remove: HashSet<T>,
}

impl<T, U> Default for AddRemoveQueues<T, U> {
    fn default() -> Self {
        Self {
            add: HashMap::default(),
            remove: HashSet::default(),
        }
    }
}
pub struct BevyQueryComponents<T: Component> {
    components: HashMap<Entity, T>,
    pub(crate) query_read: Receiver<EntityComponentQueue<T>>,
    query_write: Sender<EntityComponentQueue<T>>,
}
