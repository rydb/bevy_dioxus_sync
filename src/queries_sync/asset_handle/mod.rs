//! hook synchronization for components that are newtypes around asset handles to syncronize those underlying assets.

use std::collections::HashSet;
use std::default;
use std::{marker::PhantomData, ops::Deref};

use bevy_app::Update;
use bevy_asset::*;
use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryFilter;
use bevy_utils::default;
use crate::queries_sync::one_component_kind::{fallback_update_removed_components, AddRemoveQueues};
use crate::queries_sync::BevyDioxusIO;
use crate::*;

pub enum AssetLoadState<T> {
    Loaded(T),
    Loading,
    Failed(String)
}

pub struct RequestBevyAsset<T, U> {
    io: BevyDioxusIO<AddRemoveQueues<AssetLoadState<U>>>,
    _phantom: PhantomData<T>,
}

impl<T, U> RequestBevyAsset<T, U> 
    where
        T: Deref<Target = Handle<U>> + Component,
        U: Asset + Clone
{
        pub fn new<'a>() -> Self {
        let (bevy_tx, dioxus_rx) = crossbeam_channel::unbounded::<AddRemoveQueues<AssetLoadState<U>>>();
        let (dioxus_tx, bevy_rx) = crossbeam_channel::unbounded::<AddRemoveQueues<AssetLoadState<U>>>();

        Self {
            io: BevyDioxusIO {
                dioxus_tx,
                dioxus_rx,
                bevy_tx,
                bevy_rx,
            },
            _phantom: PhantomData,
        }
    }
}

impl<T, U> Command for RequestBevyAsset<T, U>
    where
        T: Deref<Target = Handle<U>> + Component,
        U: Asset + Clone
{
    fn apply(self, world: &mut World) -> () {
        world.insert_resource(BevyTxChannel(self.io.bevy_tx.clone()));
        world.insert_resource(BevyRxChannel(self.io.bevy_rx));

        let component_hook = world
        .register_component_hooks::<T>()
        .try_on_remove(|mut world, hook| {
            let bevy_tx = world.get_resource_mut::<BevyTxChannel<AddRemoveQueues<U>>>().unwrap();

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
            self.io.bevy_tx.send(AddRemoveQueues::<U> {
                add,
                remove: Default::default()
            }).inspect_err(|err| warn!("Could not send initial component map due to: {:#}", err));
        }
        add_systems_through_world(world, Update, send_updated_entity_components::<T>);
    }
}

/// Component that asset T has been initialized within dioxus and is being tracked
#[derive(Component)]
pub struct DioxusTrackedAsset<T, U>
    where
        T: Deref<Target = Handle<U>> + Component,
        U: Asset + Clone
{
    phantom_a: PhantomData<T>,
    phantom_b: PhantomData<U>,
}

impl<T, U> Default for DioxusTrackedAsset<T, U> 
    where
        T: Deref<Target = Handle<U>> + Component,
        U: Asset + Clone
{
    fn default() -> Self {
        Self { phantom_a: Default::default(), phantom_b: Default::default() }
    }
}

fn get_asset_status<T, U, V: QueryFilter>(
    asset_server: Res<AssetServer>,
    assets: Res<Assets<U>>,
    dioxus_asset_requests: Query<(Entity, &T), V>,
) -> HashMap<Entity, AssetLoadState<U>> 
    where
        T: Deref<Target = Handle<U>> + Component,
        U: Asset + Clone
{
    let mut map = HashMap::new();
    for (e, handle) in dioxus_asset_requests {
        let id = handle.id();
        if let Some(asset) = assets.get(id) {
            map.insert(e, AssetLoadState::Loaded(asset.clone()));
        } else {
            let load_status = asset_server.load_state(id);

            match load_status {

                LoadState::Failed(asset_load_error) => {
                    map.insert(e, AssetLoadState::Failed(asset_load_error.to_string()));

                },
                LoadState::Loading => {
                    map.insert(e, AssetLoadState::Loading);
                }
                _ => continue
            }
        }
    }
    map
}

pub fn try_load_initial_asset<T, U, V: QueryFilter>(
    asset_server: Res<AssetServer>,
    assets: Res<Assets<U>>,
    dioxus_asset_requests: Query<(Entity, &T), Without<DioxusTrackedAsset<T, U>>>,
    bevy_tx: ResMut<BevyTxChannel<AddRemoveQueues<AssetLoadState<U>>>>,
    mut commands: Commands
) 
    where
        T: Deref<Target = Handle<U>> + Component,
        U: Asset + Clone
{
    let asset_statuses = get_asset_status(asset_server, assets, dioxus_asset_requests);
    for (e, statuses) in &asset_statuses {
        match statuses {
            AssetLoadState::Loaded(_) => {
                commands.entity(*e).insert(DioxusTrackedAsset::<T, U>::default());
            },
            AssetLoadState::Loading => {
                // re-run this system until the asset is done loading.
            },
            AssetLoadState::Failed(_) => {
                commands.entity(*e).insert(DioxusTrackedAsset::<T, U>::default());
            },
        }
    }

    bevy_tx.0.send(AddRemoveQueues {
        add: asset_statuses,
        ..default()
    });
}

pub fn send_updated_asset<T, U>(
    asset_server: Res<AssetServer>,
    assets: Res<Assets<U>>,
    dioxus_asset_requests: Query<(Entity, &T), (With<DioxusTrackedAsset<T, U>>, Changed<T>)>,
    bevy_tx: ResMut<BevyTxChannel<AddRemoveQueues<AssetLoadState<U>>>>,
    mut commands: Commands
)
    where
        T: Deref<Target = Handle<U>> + Component,
        U: Asset + Clone
{
    let asset_statuses = get_asset_status(asset_server, assets, dioxus_asset_requests);

    bevy_tx.0.send(AddRemoveQueues {
        add: asset_statuses,
        ..default()
    });

}

pub fn receive_updated_asset<T: Asset>() {

}