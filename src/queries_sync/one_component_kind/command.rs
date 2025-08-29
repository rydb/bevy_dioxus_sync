use std::collections::HashSet;

use bevy_app::Update;
use bevy_ecs::prelude::*;

use crate::{queries_sync::{one_component_kind::{AddRemoveQueues, EntityComponentQueue}, BevyDioxusIO}, *};

pub struct RequestBevyComponents<T: Component> {
    pub(crate) io: BevyDioxusIO<EntityComponentQueue<T>>
}




impl<T: Component> RequestBevyComponents<T> {
        pub fn new<'a>() -> Self {
        let (bevy_tx, dioxus_rx) = crossbeam_channel::unbounded::<EntityComponentQueue<T>>();
        let (dioxus_tx, bevy_rx) = crossbeam_channel::unbounded::<EntityComponentQueue<T>>();

        Self {
            io: BevyDioxusIO {
                dioxus_tx,
                dioxus_rx,
                bevy_tx,
                bevy_rx,
            }
        }
    }
}

impl<T: Component + Clone> Command for RequestBevyComponents<T>{
    fn apply(self, world: &mut World) -> () {
        world.insert_resource(BevyTxChannel(self.io.bevy_tx.clone()));
        world.insert_resource(BevyRxChannel(self.io.bevy_rx));

        let component_hook = world
        .register_component_hooks::<T>()
        .try_on_remove(|mut world, hook| {
            let bevy_tx = world.get_resource_mut::<BevyTxChannel<EntityComponentQueue<T>>>().unwrap();

            let mut remove = HashSet::new();
            
            remove.insert(hook.entity);

            let new_requests = EntityComponentQueue {
                add: Default::default(),
                remove,
            };

            bevy_tx.0.send(new_requests).inspect_err(|err| warn!("could not send remove request for {:#} for {:#} due to {:#}", hook.entity, type_name::<T>(), err));
        });

        if component_hook.is_none() {
            warn!("could not add .on_remove hook for {:#} because it already has one, using manual system check instead", type_name::<T>());
            add_systems_through_world(world, Update, fallback_update_removed_components::<T>);
        }
        // populate initial component map set
        {
            let mut add = HashMap::default();
            let mut components = world.query::<(Entity, &T)>();

            for (e, component) in components.iter(world) {
                add.insert(e, component.clone());
            }
            self.io.bevy_tx.send(AddRemoveQueues {
                add,
                remove: Default::default()
            }).inspect_err(|err| warn!("Could not send initial component map due to: {:#}", err));
        }
        add_systems_through_world(world, Update, send_updated_entity_components::<T>);
    }
}

pub(crate) fn populate_initial_entity_component_map< T: Component + Clone>(
    components: Query<(Entity, &T)>,
    bevy_tx: ResMut<BevyTxChannel<EntityComponentQueue<T>>>,
) {
    // let mut vec = Vec::new();
    let mut map = HashMap::new();
    for (e, component) in components {
        //let x = Arc::new(component);
        map.insert(e, component.clone());
        //vec.push(component.clone())
    }

    bevy_tx.0.send(AddRemoveQueues {
        add: map,
        remove: Default::default()
    }).inspect_err(|err| warn!("Could not send component set due to: {:#}", err));
}  

pub(crate) fn send_updated_entity_components<'a, T: Component + Clone>(
    components: Query<(Entity, &T), Changed<T>>,
    bevy_tx: ResMut<BevyTxChannel<EntityComponentQueue<T>>>,
) {
    let mut map = HashMap::new();
    for (e, component) in components {
        map.insert(e, component.clone());
    }

    bevy_tx.0.send(AddRemoveQueues {
        add: map,
        remove: Default::default()
    }).inspect_err(|err| warn!("Could not send component set update due to: {:#}", err));
}  

pub(crate) fn fallback_update_removed_components<T: Component>(
    bevy_tx: ResMut<BevyTxChannel<EntityComponentQueue<T>>>,
    mut removed: RemovedComponents<T>
) {
    let mut map = HashSet::new();
    for removed in removed.read() {
        map.insert(removed);
    }
    bevy_tx.0.send(AddRemoveQueues {
        add: Default::default(),
        remove: map
    }).inspect_err(|err| warn!("could not push remove requests due to {:#}", err));
}  
