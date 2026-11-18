use std::any::{Any, TypeId};
use std::sync::Arc;

use dioxus_core::Element;
use dioxus_core_macro::rsx;
use dioxus_html::elements::prelude::*;
use dioxus_html::html;
use indexmap::IndexMap;

use super::command::{CloseSlot, MinimizeSlot, TitleOpCommand};

/// One title bar action.
pub trait TitleOp: Any + Send + Sync + 'static {
    /// Returns the clickable icon.
    fn icon(&self) -> Element;
    /// Returns the command the icon applies.
    fn command(&self) -> TitleOpCommand;
}

/// Title bar actions keyed by type.
pub struct TitleOps {
    pub ops: IndexMap<TypeId, Arc<dyn TitleOp>>,
}

impl TitleOps {
    /// Creates an empty action set.
    pub fn empty() -> Self {
        Self {
            ops: IndexMap::new(),
        }
    }

    /// Builds an action set from instances.
    pub fn from_ops(ops: impl IntoIterator<Item = Arc<dyn TitleOp>>) -> Self {
        let mut this = Self::empty();
        for op in ops {
            let id = op.as_ref().type_id();
            this.ops.insert(id, op);
        }
        this
    }

    /// Inserts an action instance.
    pub fn insert<T: TitleOp>(&mut self, op: T) {
        self.ops.insert(TypeId::of::<T>(), Arc::new(op));
    }

    /// Returns whether an action type is present.
    pub fn contains<T: TitleOp>(&self) -> bool {
        self.ops.contains_key(&TypeId::of::<T>())
    }

    /// Returns the action instance for a type.
    pub fn get<T: TitleOp>(&self) -> Option<&Arc<dyn TitleOp>> {
        self.ops.get(&TypeId::of::<T>())
    }

    /// Iterates actions in display order.
    pub fn iter(&self) -> impl Iterator<Item = (TypeId, &Arc<dyn TitleOp>)> + '_ {
        self.ops.iter().map(|(id, op)| (*id, op))
    }

    /// Returns the number of actions.
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Returns whether no actions are present.
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

impl Clone for TitleOps {
    fn clone(&self) -> Self {
        Self {
            ops: self.ops.clone(),
        }
    }
}

impl PartialEq for TitleOps {
    fn eq(&self, other: &Self) -> bool {
        self.ops.len() == other.ops.len() && self.ops.keys().eq(other.ops.keys())
    }
}

impl Eq for TitleOps {}

impl std::fmt::Debug for TitleOps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TitleOps")
            .field("ops", &self.ops.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Default for TitleOps {
    fn default() -> Self {
        Self::from_ops([
            Arc::new(Minimize) as Arc<dyn TitleOp>,
            Arc::new(Close) as Arc<dyn TitleOp>,
        ])
    }
}

/// Shrinks the hosting slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Minimize;

impl TitleOp for Minimize {
    fn icon(&self) -> Element {
        rsx! {
            span { "-" }
        }
    }

    fn command(&self) -> TitleOpCommand {
        TitleOpCommand::slot(MinimizeSlot)
    }
}

/// Closes the hosting slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Close;

impl TitleOp for Close {
    fn icon(&self) -> Element {
        rsx! {
            span { "x" }
        }
    }

    fn command(&self) -> TitleOpCommand {
        TitleOpCommand::surface(CloseSlot)
    }
}
