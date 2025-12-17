use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    sync::Arc,
};

use bevy_app::Update;
use bevy_dioxus_interop::{
    add_systems_through_world,
    signals::CrossDomSignal,
    traits::{CrossDomSignalErasedMap, ErasedSignal},
};
use bevy_ecs::prelude::*;
use bevy_ecs::{
    component::Mutable,
    query::{QueryData, QueryFilter},
    resource::Resource,
    system::Command,
};
use bevy_log::warn;
use bytemuck::TransparentWrapper;

#[derive(TransparentWrapper, Resource, Default)]
#[repr(transparent)]
pub struct BevyQuerySignals(HashMap<TypeId, ErasedSignal>);

impl CrossDomSignalErasedMap for BevyQuerySignals {
    type Index = TypeId;

    type AdditionalInfo = ();
}

#[derive(TransparentWrapper)]
#[repr(transparent)]
#[transparent(DioxusQueryResults<T>)]
pub struct RequestBevyQuery<T: Clone + DioxusQueryData, U>(
    // Query Data
    DioxusQueryResults<T>,
    // Query Filter
    PhantomData<U>,
);

#[derive(Resource, Default)]
pub struct AddedComponentClones(HashSet<TypeId>);

#[derive(Resource, Default)]
pub struct AddedQueries(HashSet<TypeId>);

impl<T, U> Command for RequestBevyQuery<T, U>
where
    T: DioxusQueryData + Clone + Send + Sync + 'static,
    U: QueryFilter + Send + Sync + 'static,
{
    fn apply(self, world: &mut bevy_ecs::world::World) -> () {
        // world.init_resource::<DioxusQueryResults<T::DioxusItem>>();

        match world.get_resource::<DioxusQueryResults<T>>() {
            Some(n) => self.0.0.pnt_to(n.0.get_ptr().unwrap()),
            None => {
                self.0.0.initialize(HashMap::new());
                world.insert_resource(self.0.clone());
            }
        }
        let mut added_queries = world.get_resource_or_init::<AddedQueries>();

        if added_queries.0.contains(&TypeId::of::<T>()) == false {
            added_queries.0.insert(TypeId::of::<T>());

            add_systems_through_world(world, Update, update_query_signal_results::<T, U>);
        }
        T::register_component_sync_systems(world);
    }
}

#[derive(Clone, Component)]
pub struct DioxusClone<T: Component>(pub CrossDomSignal<T>);

pub fn sync_bevy_to_clone<T: Component<Mutability = Mutable> + Clone>(
    original_query: Query<(Entity, &T), Changed<T>>,
    mut clone_query: Query<(Entity, &mut DioxusClone<T>)>,
    mut commands: Commands,
) {
    for (e, update) in original_query {
        let Ok((_, value)) = clone_query.get_mut(e).inspect_err(|err| warn!("{err}")) else {
            commands.entity(e).insert(DioxusClone(CrossDomSignal::new(update.clone())));
            let entities = clone_query.iter().map(|(e, _)| e).collect::<Vec<_>>();
            warn!("entities that do exist.. {:#?}", entities);
        
            continue;
        };
        let _ = value.0.swap(Arc::new(update.clone())).inspect_err(|err| warn!("{err}"));
    }
}

pub fn sync_dioxus_clone<T: Component<Mutability = Mutable> + Clone>(
    clone_query: Query<(Entity, &DioxusClone<T>), Changed<DioxusClone<T>>>,
    mut original_query: Query<&mut T>,
) {
    for (e, update) in clone_query {
        let Ok(mut component) = original_query.get_mut(e).inspect_err(|err| warn!("{err}")) else {
            continue;
        };
        let value = update.0.get().unwrap().as_ref().clone();
        *component = value
    }
}

#[derive(Resource)]
pub struct DioxusQuery<T: DioxusQueryData> {
    /// bevy [`QueryData`] as its type id
    pub query_id: TypeId,
    pub query_info: Vec<T::DioxusItem>,
}

impl<T: DioxusQueryData + 'static> DioxusQuery<T> {
    pub fn new<'w, 's>(query_results: Vec<T::Item<'w, 's>>) -> Self {
        let mut query_info = Vec::new();
        for item in query_results {
            query_info.push(T::wrap_as_dioxus_signals(item));
        }
        Self {
            query_id: TypeId::of::<T>(),
            query_info,
        }
    }
}

/// Query super trait for logistics for bevy <-> dioxus interop
///
/// NOTE: [`Entity`] must always be the first bound of a query due to needing to cache `DioxusClone`(s) with it
pub trait DioxusQueryData: QueryData {
    type DioxusItem: Clone + Send + Sync + 'static;
    type DioxusCloneQuery: QueryData;

    fn spawn_dioxus_signals(
        mut commands: Commands,
        entity: Entity,
        signal_components: Self::DioxusItem,
    ) {
        commands
            .entity(entity)
            .insert(Self::get_bundle(signal_components));
    }

    fn register_component_sync_systems(world: &mut World);
    /// clones a bevy component into a pointer that a dioxus signal can read from.
    fn wrap_as_dioxus_signals<'w, 's>(item: Self::Item<'w, 's>) -> Self::DioxusItem;

    fn get_entity<'w, 's>(
        item: &<<Self as DioxusQueryData>::DioxusCloneQuery as QueryData>::Item<'w, 's>,
    ) -> Entity;
    fn get_bundle<'w, 's>(item: Self::DioxusItem) -> impl Bundle;
    fn clone_dioxus_signals<'w, 's>(
        item: <<Self as DioxusQueryData>::DioxusCloneQuery as QueryData>::Item<'w, 's>,
    ) -> Self::DioxusItem;
}

impl<A: Component<Mutability = Mutable> + Clone> DioxusQueryData for (Entity, &A) {
    type DioxusItem = (Entity, DioxusClone<A>);
    type DioxusCloneQuery = (Entity, &'static DioxusClone<A>);

    fn wrap_as_dioxus_signals<'w, 's>(item: Self::Item<'w, 's>) -> Self::DioxusItem {
        (
            item.0,
            DioxusClone(CrossDomSignal::new(item.1.clone())),
        )
    }
    fn register_component_sync_systems(world: &mut World) {
        let mut added_components = world.get_resource_or_init::<AddedComponentClones>();

        if added_components.0.contains(&TypeId::of::<A>()) == false {
            added_components.0.insert(TypeId::of::<A>());

            add_systems_through_world(
                world,
                Update,
                (sync_dioxus_clone::<A>, sync_bevy_to_clone::<A>),
            );
        }
    }

    fn clone_dioxus_signals<'w, 's>(
        item: <<Self as DioxusQueryData>::DioxusCloneQuery as QueryData>::Item<'w, 's>,
    ) -> Self::DioxusItem {
        (item.0, item.1.clone())
    }

    fn get_entity<'w, 's>(
        item: &<<Self as DioxusQueryData>::DioxusCloneQuery as QueryData>::Item<'w, 's>,
    ) -> Entity {
        item.0
    }

    fn get_bundle<'w, 's>(item: Self::DioxusItem) -> impl Bundle {
        item.1
    }
}
#[derive(TransparentWrapper, Resource, Clone, Default)]
#[repr(transparent)]
pub struct DioxusQueryResults<T: Clone + DioxusQueryData>(
    CrossDomSignal<HashMap<Entity, T::DioxusItem>>,
);

// impl<T> Display for DioxusQueryResults<T>
//     where
//         T: Clone + DioxusQueryData,
//         CrossDomSignal<HashMap<Entity, T::DioxusItem>>: Display
// {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let string = format!("{:#}", self.0);
//         write!(f, "{string}", )
//     }
// }

pub fn update_query_signal_results<T: DioxusQueryData + Clone + 'static, U: QueryFilter>(
    query_results: Query<T::DioxusCloneQuery, U>,
    query_resource: ResMut<DioxusQueryResults<T>>,
) {
    //TODO: figure out how to insert new entries into the hasmap instead of cloning the entire map, appending the new entry to the clone, then swapping the old map with the new one...

    let binding = query_resource.0.get().unwrap();
    let cached_entries = binding.keys().collect::<Vec<_>>();
    for n in &cached_entries {
        if query_results.contains(**n) == false {
            let mut old_entries = query_resource
                .0
                .get()
                .unwrap()
                .iter()
                .map(|n| (n.0.clone(), n.1.clone()))
                .collect::<HashMap<_, _>>();

            old_entries.remove(n);

            let _ = query_resource.0.swap(old_entries.into());
        }
    }
    for n in query_results {
        let e = T::get_entity(&n);
        if cached_entries.contains(&&e) == false {
            let mut old_entries = query_resource
                .0
                .get()
                .unwrap()
                .iter()
                .map(|n| (n.0.clone(), n.1.clone()))
                .collect::<HashMap<_, _>>();
            let new_entry = (e.clone(), T::clone_dioxus_signals(n));

            old_entries.insert(new_entry.0, new_entry.1);

            let _ = query_resource.0.swap(old_entries.into());
        }
    }
}
