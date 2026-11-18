use std::any::TypeId;

use dioxus_core::Element;
use indexmap::IndexMap;

pub use derive_name::Name;

/// Renders one tab body.
pub type TabFn = fn() -> Element;

/// Static tab identity and content.
pub trait Tab: Send + Sync + 'static + Name {
    /// Returns the body renderer.
    fn element() -> TabFn;
}

/// One registered tab: body renderer and title.
#[derive(Clone, Copy, Debug)]
pub struct TabEntry {
    /// Body renderer.
    pub render: TabFn,
    /// Title shown in the title bar.
    pub title: &'static str,
}

impl TabEntry {
    /// Builds the entry for a tab type.
    pub fn new<T: Tab>() -> Self {
        Self {
            render: T::element(),
            title: <T as Name>::name(),
        }
    }
}

impl PartialEq for TabEntry {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title && std::ptr::fn_addr_eq(self.render, other.render)
    }
}

impl Eq for TabEntry {}

/// Returns the group entry for a tab type.
pub fn tab_entry<T: Tab>() -> (TypeId, TabEntry) {
    (TypeId::of::<T>(), TabEntry::new::<T>())
}

/// The tab rendered inside a slot.
#[derive(Clone, Debug)]
pub struct ActiveTab {
    pub id: TypeId,
    pub element: TabFn,
}

impl PartialEq for ActiveTab {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ActiveTab {}

impl ActiveTab {
    /// Creates the active tab for a type.
    pub fn new<T: Tab>() -> Self {
        let (id, entry) = tab_entry::<T>();
        Self {
            id,
            element: entry.render,
        }
    }

    /// Replaces the active tab with another type.
    pub fn set<T: Tab>(&mut self) {
        *self = Self::new::<T>();
    }
}

/// How inactive tabs are kept alive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabMount {
    /// Every tab stays mounted.
    #[default]
    Eager,
    /// A tab stays mounted after it is selected once.
    LazyKeepAlive,
    /// Only the selected tab stays mounted.
    UnmountOnDeselect,
}

/// Selector for the tabs hosted by one title bar.
#[derive(Clone, Debug)]
pub struct TabGroup {
    pub tabs: IndexMap<TypeId, TabEntry>,
    pub mount: TabMount,
}

impl PartialEq for TabGroup {
    fn eq(&self, other: &Self) -> bool {
        self.mount == other.mount
            && self.tabs.len() == other.tabs.len()
            && self.tabs.keys().eq(other.tabs.keys())
    }
}

impl Eq for TabGroup {}

impl TabGroup {
    /// Builds a group from tab entries.
    pub fn from_entries(
        entries: impl IntoIterator<Item = (TypeId, TabEntry)>,
        mount: TabMount,
    ) -> Self {
        Self {
            tabs: entries.into_iter().collect(),
            mount,
        }
    }

    /// Returns whether a tab type belongs to the group.
    pub fn contains(&self, id: TypeId) -> bool {
        self.tabs.contains_key(&id)
    }

    /// Returns whether a tab type belongs to the group.
    pub fn contains_type<T: Tab>(&self) -> bool {
        self.contains(TypeId::of::<T>())
    }

    /// Returns the renderer for a tab type.
    pub fn element(&self, id: TypeId) -> Option<TabFn> {
        self.tabs.get(&id).map(|entry| entry.render)
    }

    /// Returns the title for a tab type.
    pub fn title(&self, id: TypeId) -> Option<&'static str> {
        self.tabs.get(&id).map(|entry| entry.title)
    }

    /// Selects a tab by type.
    pub fn select<T: Tab>(&self) -> Option<ActiveTab> {
        self.select_id(TypeId::of::<T>())
    }

    /// Selects a tab by identity.
    pub fn select_id(&self, id: TypeId) -> Option<ActiveTab> {
        self.element(id).map(|element| ActiveTab { id, element })
    }

    /// Selects the first tab in the group.
    pub fn first(&self) -> Option<ActiveTab> {
        self.tabs.iter().next().map(|(id, entry)| ActiveTab {
            id: *id,
            element: entry.render,
        })
    }

    /// Returns the number of tabs.
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// Returns whether the group has no tabs.
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// Returns every tab identity in display order.
    pub fn ids(&self) -> impl Iterator<Item = TypeId> + '_ {
        self.tabs.keys().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_core::VNode;

    fn body() -> Element {
        Ok(VNode::placeholder())
    }

    #[derive(Name)]
    struct First;
    #[derive(Name)]
    struct Second;

    impl Tab for First {
        fn element() -> TabFn {
            body
        }
    }

    impl Tab for Second {
        fn element() -> TabFn {
            body
        }
    }

    #[test]
    fn tab_entry_reports_the_type_name() {
        let (id, entry) = tab_entry::<First>();
        assert_eq!(id, TypeId::of::<First>());
        assert_eq!(entry.title, "First");
    }

    #[test]
    fn group_selects_a_member() {
        let group = TabGroup::from_entries(
            [tab_entry::<First>(), tab_entry::<Second>()],
            TabMount::Eager,
        );
        let active = group.select::<Second>().unwrap();
        assert_eq!(active.id, TypeId::of::<Second>());
        assert!(group.select::<First>().is_some());
    }
}
