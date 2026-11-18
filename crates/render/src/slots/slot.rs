use std::any::TypeId;

use super::float::FloatPanels;
use super::percent::Percent;
use super::solver::{EDGE_SHARE, Viewport, edge_bounds};
use super::tab::{ActiveTab, Tab, TabGroup};
use super::theme::Theme;
use super::title::TitleOps;

/// Side of the five slot layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlotSide {
    Top,
    Left,
    Right,
    Bottom,
    Center,
}

impl SlotSide {
    /// Every side in layout order.
    pub const ALL: [SlotSide; 5] = [
        SlotSide::Top,
        SlotSide::Left,
        SlotSide::Right,
        SlotSide::Bottom,
        SlotSide::Center,
    ];

    /// Returns the side index used for stable ordering.
    pub const fn index(self) -> usize {
        match self {
            Self::Top => 0,
            Self::Left => 1,
            Self::Right => 2,
            Self::Bottom => 3,
            Self::Center => 4,
        }
    }
}

/// Which pair of edges spans the full surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EdgeFillRule {
    /// Left and right span the full height.
    #[default]
    LeftRight,
    /// Top and bottom span the full width.
    TopBottom,
    /// Top spans the full width, right starts below the top.
    TopRight,
    /// Top spans the full width, left starts below the top.
    TopLeft,
    /// Bottom spans the full width, right stops above the bottom.
    BottomRight,
    /// Bottom spans the full width, left stops above the bottom.
    BottomLeft,
}

/// How an edge reacts to a drag on its boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EdgeResizeRule {
    /// Resizes on center slot adjacent edge drag, minimizes when dragged to edge of surface
    ResizeNoMinimize,
    /// Resizes + minimizes/expands when dragged to and from surface surface edges
    #[default]
    Resize,
}

/// Resize rule per edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EdgeResize {
    pub top: EdgeResizeRule,
    pub right: EdgeResizeRule,
    pub bottom: EdgeResizeRule,
    pub left: EdgeResizeRule,
}

impl EdgeResize {
    /// Returns the rule for an edge side.
    pub fn rule(&self, side: SlotSide) -> EdgeResizeRule {
        match side {
            SlotSide::Top => self.top,
            SlotSide::Right => self.right,
            SlotSide::Bottom => self.bottom,
            SlotSide::Left => self.left,
            SlotSide::Center => EdgeResizeRule::Resize,
        }
    }
}

/// Share of each axis taken by an edge slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeSizes {
    pub top: Percent,
    pub right: Percent,
    pub bottom: Percent,
    pub left: Percent,
}

impl Default for EdgeSizes {
    fn default() -> Self {
        Self {
            top: EDGE_SHARE,
            right: EDGE_SHARE,
            bottom: EDGE_SHARE,
            left: EDGE_SHARE,
        }
    }
}

impl EdgeSizes {
    /// Returns the share of an edge side.
    pub fn get(&self, side: SlotSide) -> Percent {
        match side {
            SlotSide::Top => self.top,
            SlotSide::Right => self.right,
            SlotSide::Bottom => self.bottom,
            SlotSide::Left | SlotSide::Center => self.left,
        }
    }

    /// Returns the share of an edge side mutably.
    pub fn get_mut(&mut self, side: SlotSide) -> &mut Percent {
        match side {
            SlotSide::Top => &mut self.top,
            SlotSide::Right => &mut self.right,
            SlotSide::Bottom => &mut self.bottom,
            SlotSide::Left | SlotSide::Center => &mut self.left,
        }
    }
}

impl EdgeFillRule {
    /// Returns whether an edge is one of the two winning edges.
    pub fn fills(self, side: SlotSide) -> bool {
        matches!(
            (self, side),
            (Self::LeftRight, SlotSide::Left | SlotSide::Right)
                | (Self::TopBottom, SlotSide::Top | SlotSide::Bottom)
                | (Self::TopLeft, SlotSide::Top | SlotSide::Left)
                | (Self::TopRight, SlotSide::Top | SlotSide::Right)
                | (Self::BottomLeft, SlotSide::Bottom | SlotSide::Left)
                | (Self::BottomRight, SlotSide::Bottom | SlotSide::Right)
        )
    }

    /// Returns the start and length of an edge along its own axis.
    pub(crate) fn axis_span(
        self,
        side: SlotSide,
        extent: u32,
        start_inset: u32,
        end_inset: u32,
    ) -> (i32, u32) {
        if !self.fills(side) {
            let length = extent.saturating_sub(start_inset).saturating_sub(end_inset);
            return (start_inset as i32, length);
        }
        match self {
            Self::TopLeft => match side {
                SlotSide::Left => (start_inset as i32, extent.saturating_sub(start_inset)),
                _ => (0, extent),
            },
            Self::TopRight => match side {
                SlotSide::Right => (start_inset as i32, extent.saturating_sub(start_inset)),
                _ => (0, extent),
            },
            Self::BottomLeft => match side {
                SlotSide::Left => (0, extent.saturating_sub(end_inset)),
                _ => (0, extent),
            },
            Self::BottomRight => match side {
                SlotSide::Right => (0, extent.saturating_sub(end_inset)),
                _ => (0, extent),
            },
            Self::LeftRight | Self::TopBottom => (0, extent),
        }
    }
}

/// Title bar owned by a slot.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum TitleBar {
    /// The slot has no title bar and cannot be moved.
    #[default]
    Disabled,
    /// The slot shows tabs and actions.
    Enabled { group: TabGroup, ops: TitleOps },
}

impl TitleBar {
    /// Returns the tab group when the title bar is enabled.
    pub fn group(&self) -> Option<&TabGroup> {
        match self {
            Self::Disabled => None,
            Self::Enabled { group, .. } => Some(group),
        }
    }

    /// Returns the actions when the title bar is enabled.
    pub fn ops(&self) -> Option<&TitleOps> {
        match self {
            Self::Disabled => None,
            Self::Enabled { ops, .. } => Some(ops),
        }
    }

    /// Returns whether the title bar hosts a tab type.
    pub fn contains(&self, id: TypeId) -> bool {
        self.group().is_some_and(|group| group.contains(id))
    }

    /// Builds the active tab for a type.
    pub fn select_id(&self, id: TypeId) -> Option<ActiveTab> {
        self.group().and_then(|group| group.select_id(id))
    }
}

/// Display state of one slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SlotState {
    /// The slot is mounted and visible.
    #[default]
    Open,
    /// The slot is unmounted but keeps its tabs.
    Closed,
    /// The slot is mounted but hidden behind an edge handle.
    Minimized,
}

/// One slot: a title bar selector, the single active tab, and its display state.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Slot {
    pub title_bar: TitleBar,
    pub active_tab: Option<ActiveTab>,
    pub state: SlotState,
}

impl Slot {
    /// Returns whether the slot takes space in layout.
    pub fn is_occupied(&self) -> bool {
        self.state == SlotState::Open && (self.active_tab.is_some() || self.has_tabs())
    }

    /// Returns whether the slot has no active tab.
    pub fn is_empty(&self) -> bool {
        self.active_tab.is_none()
    }

    /// Returns whether the title bar hosts at least one tab.
    pub fn has_tabs(&self) -> bool {
        self.title_bar
            .group()
            .is_some_and(|group| !group.is_empty())
    }

    /// Selects a tab by identity.
    pub fn select_id(&mut self, id: TypeId) -> bool {
        let Some(active) = self.title_bar.select_id(id) else {
            return false;
        };
        self.active_tab = Some(active);
        true
    }

    /// Selects a tab by type.
    pub fn select<T: Tab>(&mut self) -> bool {
        self.select_id(TypeId::of::<T>())
    }

    /// Returns every tab identity in display order.
    pub fn tab_ids(&self) -> Vec<TypeId> {
        self.title_bar
            .group()
            .map(|group| group.ids().collect())
            .unwrap_or_default()
    }

    /// Returns the mounted tab identities for the current mount policy.
    pub fn mounted_tabs(&self) -> Vec<TypeId> {
        let Some(group) = self.title_bar.group() else {
            return Vec::new();
        };
        let active = self.active_tab.as_ref().map(|tab| tab.id);
        group
            .ids()
            .filter(|id| match group.mount {
                super::tab::TabMount::Eager => true,
                super::tab::TabMount::LazyKeepAlive => true,
                super::tab::TabMount::UnmountOnDeselect => Some(*id) == active,
            })
            .collect()
    }

    /// Returns the active tab identity.
    pub fn active_id(&self) -> Option<TypeId> {
        self.active_tab.as_ref().map(|tab| tab.id)
    }
}

/// Five docked slots plus the floating collection.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FiveSlots {
    pub top: Slot,
    pub left: Slot,
    pub right: Slot,
    pub bottom: Slot,
    pub center: Slot,
    pub float_panels: FloatPanels,
    pub edge_fill_rule: EdgeFillRule,
    pub edge_sizes: EdgeSizes,
    pub edge_resize: EdgeResize,
    pub theme: Theme,
}

impl FiveSlots {
    /// Returns a slot by side.
    pub fn slot(&self, side: SlotSide) -> &Slot {
        match side {
            SlotSide::Top => &self.top,
            SlotSide::Left => &self.left,
            SlotSide::Right => &self.right,
            SlotSide::Bottom => &self.bottom,
            SlotSide::Center => &self.center,
        }
    }

    /// Returns a mutable slot by side.
    pub fn slot_mut(&mut self, side: SlotSide) -> &mut Slot {
        match side {
            SlotSide::Top => &mut self.top,
            SlotSide::Left => &mut self.left,
            SlotSide::Right => &mut self.right,
            SlotSide::Bottom => &mut self.bottom,
            SlotSide::Center => &mut self.center,
        }
    }

    /// Resizes one edge from a pixel start and signed delta.
    pub fn resize_edge(
        &mut self,
        side: SlotSide,
        viewport: Viewport,
        start_px: u32,
        delta_px: i32,
    ) {
        if side == SlotSide::Center {
            return;
        }
        let rule = self.edge_resize.rule(side);
        let raw = start_px as i32 + delta_px;
        if rule == EdgeResizeRule::Resize && raw <= 0 {
            self.slot_mut(side).state = SlotState::Minimized;
            return;
        }
        let (min, max) = edge_bounds(side, self, viewport);
        let size = raw.clamp(min as i32, max as i32) as u32;
        let axis = match side {
            SlotSide::Top | SlotSide::Bottom => viewport.height_px,
            SlotSide::Left | SlotSide::Right => viewport.width_px,
            SlotSide::Center => return,
        };
        *self.edge_sizes.get_mut(side) = Percent::from_px(size as i32, axis);
        if rule == EdgeResizeRule::Resize {
            self.slot_mut(side).state = SlotState::Open;
        }
    }

    /// Selects a tab wherever it is hosted.
    pub fn select_id(&mut self, id: TypeId) -> bool {
        for side in SlotSide::ALL {
            if self.slot_mut(side).select_id(id) {
                return true;
            }
        }
        self.float_panels.select_by_tab(id)
    }

    /// Returns the side that hosts a tab.
    pub fn side_of(&self, id: TypeId) -> Option<SlotSide> {
        SlotSide::ALL
            .into_iter()
            .find(|side| self.slot(*side).title_bar.contains(id))
    }
}
