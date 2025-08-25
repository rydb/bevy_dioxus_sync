use std::{any::type_name, collections::{HashMap, HashSet}, sync::Arc, thread::scope};

use async_std::task::sleep;
use bevy_app::Update;
use bevy_ecs::{prelude::*, query::QueryData, world::CommandQueue};
use bevy_log::{info, warn};
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use dioxus::{hooks::{use_context, use_future}, signals::{Signal, SignalSubscriberDrop, SyncSignal, UnsyncStorage, WritableExt, WriteLock}};

use crate::{add_systems_through_world, dioxus_in_bevy_plugin::DioxusProps, queries_sync::BevyDioxusIO, traits::ErasedSubGenericComponentsMap, BevyRxChannel, BevyTxChannel, BoxAnyTypeMap};

pub struct RequestBevyComponents<T: Component> {
    pub(crate) io: BevyDioxusIO<AddRemoveQueues<T>>
}




impl<T: Component> RequestBevyComponents<T> {
        pub fn new<'a>() -> Self {
        let (bevy_tx, dioxus_rx) = crossbeam_channel::unbounded::<AddRemoveQueues<T>>();
        let (dioxus_tx, bevy_rx) = crossbeam_channel::unbounded::<AddRemoveQueues<T>>();

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
            let bevy_tx = world.get_resource_mut::<BevyTxChannel<AddRemoveQueues<T>>>().unwrap();

            let mut remove = HashSet::new();
            
            remove.insert(hook.entity);

            let new_requests = AddRemoveQueues {
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

pub struct AddRemoveQueues<T> {
    pub add: HashMap<Entity, T>,
    pub remove: HashSet<Entity>
}

impl<T> Default for AddRemoveQueues<T> {
    fn default() -> Self {
        Self { add: HashMap::default(), remove: HashSet::default() }
    }
}

pub(crate) fn populate_initial_entity_component_map< T: Component + Clone>(
    components: Query<(Entity, &T)>,
    bevy_tx: ResMut<BevyTxChannel<AddRemoveQueues<T>>>,
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
    bevy_tx: ResMut<BevyTxChannel<AddRemoveQueues<T>>>,
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
    bevy_tx: ResMut<BevyTxChannel<AddRemoveQueues<T>>>,
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


pub enum BevyQueryScope {
    Entity(Entity),
    World
}

// pub struct BevyQuery<T: Component + Clone> {
//     /// Filter and components being checked for
//     content: T,
// }


// pub fn changed<T: Component + Clone>(
//     query: Query<Ref<T>>,
// ) {
//     let sample = BevyQueryScope::World;

//     match sample {
//         BevyQueryScope::Entity(entity) => {
//             let Ok(data) =query.get(entity)
//             .inspect_err(|err| warn!("blah blah blah thing doesn't exist")) else {
//                 todo!("something somehting something, tell dioxus that entity is invalid and to requery for entity based on component or something");
//                 (entity, data);
//                 return;
//             }
//         },
//         BevyQueryScope::World => {f
//             let datas = query.iter().clone();
//                 todo!("send this to dioxus for it to read inside BevyQuery or somethign and update its state with this component")
//                 datas

//             },
//     }
// }


#[derive(TransparentWrapper, Default)]
#[repr(transparent)]
pub struct ComponentsErased(BoxAnyTypeMap);

impl ErasedSubGenericComponentsMap for ComponentsErased {
    type Generic<T: Clone + Component + Send + Sync + 'static> = SyncSignal<BevyQueryComponents<T>>;
}

#[derive(Clone, Default)]
pub struct BevyComponentsSignals(Signal<ComponentsErased>);

pub struct BevyQueryComponents<T: Component> {
    pub components: HashMap<Entity, T>,
    pub query_read: Receiver<AddRemoveQueues<T>>,
    pub query_write: Sender<AddRemoveQueues<T>>,
}


fn request_component_channels<T: Component + Clone>(
    props: DioxusProps,
    mut signal_registry: WriteLock<
        '_,
        ComponentsErased,
        UnsyncStorage,
        SignalSubscriberDrop<ComponentsErased, UnsyncStorage>,
    >,
) -> SyncSignal<BevyQueryComponents<T>> {
    let mut commands = CommandQueue::default();

    let command = RequestBevyComponents::<T>::new();

    let dioxus_rx = command.io.dioxus_rx.clone();
    let dioxus_tx = command.io.dioxus_tx.clone();
    commands.push(command);

    let new_signal = SyncSignal::new_maybe_sync(BevyQueryComponents {
        components: HashMap::default(),
        query_read: dioxus_rx,
        query_write: dioxus_tx,
    });

    signal_registry.insert(new_signal.clone());
    props.command_queues_tx.send(commands);

    return new_signal;
}

pub fn use_bevy_component_query<T: Component + Clone>() -> SyncSignal<BevyQueryComponents<T>> {
    let props = use_context::<DioxusProps>();

    let mut components_signals = use_context::<BevyComponentsSignals>();

    let signal = {
        let mut components = components_signals.0.write();

        let value = components.get::<T>();
        let Some(signal) = value else {
            warn!("requesting resource channel");
            return request_component_channels(props, components);
        };
        signal.clone()
    };

    use_future(move || {
        let value = props.clone();
        async move {
            let mut signal = signal.clone();
            loop {
                sleep(std::time::Duration::from_millis(1000)).await;

                let mut copies = signal.write();
                warn!("attempting to receive resource");
                while let Ok(value) = copies.query_read.try_recv() {
                    // warn!("received entity component map");
                    copies.components.retain(|key, n| value.remove.contains(key) == false);
                    copies.components.extend(value.add);
                }
            }
        }
    });
    signal

}