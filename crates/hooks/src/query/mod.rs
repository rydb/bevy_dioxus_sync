use bevy_dioxus_interop::signals::CrossDomSignal;
use bevy_ecs::{entity::Entity, query::QueryFilter};
use std::collections::HashMap;

use crate::{
    query::command::{DioxusQueryData, DioxusQueryResults, RequestBevyQuery},
    use_bevy_value,
};

pub mod command;

/// A hook to interface with bevy queries from dioxus.
///
/// NOTE: the first bound of the query must be [`Entity`]
pub fn use_bevy_query<T, U>() -> CrossDomSignal<HashMap<Entity, <T as DioxusQueryData>::DioxusItem>>
where
    T: DioxusQueryData + Clone + Send + Sync + 'static,
    U: QueryFilter + Send + Sync + 'static,
{
    use_bevy_value::<
        T,
        DioxusQueryResults<T>,
        RequestBevyQuery<T, U>,
        HashMap<Entity, <T as DioxusQueryData>::DioxusItem>,
    >()
}
