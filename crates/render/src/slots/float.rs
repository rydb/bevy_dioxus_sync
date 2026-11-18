use std::any::TypeId;

use indexmap::IndexMap;

use super::percent::{Percent, PercentSize};
use super::slot::{Slot, TitleBar};
use super::solver::{
    RectPx, ResizeEdge, Vec2Px, Viewport, anchored_rect, clamp_inside, clamp_size, clamp_visible,
    float_anchor, size_from_px,
};
use super::tab::Tab;

/// Placement anchor of a floating slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreePanelPosition {
    /// Anchored to the top left of the surface.
    TopLeft,
    /// Anchored to the top right of the surface.
    TopRight,
    /// Centered on the surface.
    Center,
    /// Anchored to the bottom left of the surface.
    BottomLeft,
    /// Anchored to the bottom right of the surface.
    BottomRight,
    /// Free percent offset from the top left of the surface.
    #[doc(hidden)]
    Offset(Percent, Percent),
}

impl Default for FreePanelPosition {
    fn default() -> Self {
        Self::Center
    }
}

/// One floating slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FloatPanel {
    pub collides_with_edge_slots: bool,
    pub resizable: bool,
    pub position: FreePanelPosition,
    pub size: PercentSize,
    pub size_min: PercentSize,
    pub size_max: PercentSize,
    pub slot: Slot,
}

impl Default for FloatPanel {
    fn default() -> Self {
        Self {
            collides_with_edge_slots: false,
            resizable: true,
            position: FreePanelPosition::Center,
            size: PercentSize::new(30, 30),
            size_min: PercentSize::new(12, 8),
            size_max: PercentSize::new(100, 100),
            slot: Slot::default(),
        }
    }
}

impl FloatPanel {
    /// Returns the percent offset of the top left corner.
    pub fn offset(&self) -> (Percent, Percent) {
        let width = self.size.width;
        let height = self.size.height;
        match self.position {
            FreePanelPosition::TopLeft => (Percent::new(0), Percent::new(0)),
            FreePanelPosition::TopRight => (Percent::new(100 - width), Percent::new(0)),
            FreePanelPosition::Center => (
                Percent::new((100 - width) / 2),
                Percent::new((100 - height) / 2),
            ),
            FreePanelPosition::BottomLeft => (Percent::new(0), Percent::new(100 - height)),
            FreePanelPosition::BottomRight => {
                (Percent::new(100 - width), Percent::new(100 - height))
            }
            FreePanelPosition::Offset(x, y) => (x, y),
        }
    }

    /// Resolves the pixel rectangle against the surface.
    pub fn rect(&self, bounds: RectPx, viewport: Viewport) -> RectPx {
        let (x, y) = self.offset();
        anchored_rect(
            self.size,
            x,
            y,
            self.collides_with_edge_slots,
            bounds,
            viewport,
        )
    }

    /// Moves the floating slot from a gesture origin.
    pub fn drag(&mut self, bounds: RectPx, viewport: Viewport, start: RectPx, delta: Vec2Px) {
        if matches!(self.slot.title_bar, TitleBar::Disabled) {
            return;
        }
        let moved = RectPx {
            x: start.x.saturating_add(delta.x),
            y: start.y.saturating_add(delta.y),
            ..start
        };
        let resolved = if self.collides_with_edge_slots {
            clamp_inside(moved, bounds)
        } else {
            clamp_visible(moved, viewport)
        };
        // Percent offsets cannot be negative, so the surface edge is the lower bound.
        let resolved = RectPx {
            x: resolved.x.max(bounds.x),
            y: resolved.y.max(bounds.y),
            ..resolved
        };
        let resolved = clamp_inside(resolved, float_anchor(viewport));
        self.position = FreePanelPosition::Offset(
            Percent::from_px(resolved.x - bounds.x, bounds.w),
            Percent::from_px(resolved.y - bounds.y, bounds.h),
        );
    }

    /// Resizes the floating slot by a pointer delta on an edge.
    pub fn resize(
        &mut self,
        bounds: RectPx,
        viewport: Viewport,
        start: RectPx,
        edge: ResizeEdge,
        delta: Vec2Px,
    ) {
        if !self.resizable {
            return;
        }
        let rect = start;
        let min_w = self.size_min.width_percent().as_px(bounds.w).max(1) as i32;
        let min_h = self.size_min.height_percent().as_px(bounds.h).max(1) as i32;
        let max_w = self.size_max.width_percent().as_px(bounds.w).max(1) as i32;
        let max_h = self.size_max.height_percent().as_px(bounds.h).max(1) as i32;
        let mut left = rect.x;
        let mut right = rect.right();
        let mut top = rect.y;
        let mut bottom = rect.bottom();

        match edge {
            ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft => {
                left = (left + delta.x).clamp(0, right - min_w);
            }
            ResizeEdge::Right | ResizeEdge::TopRight | ResizeEdge::BottomRight => {
                right = (right + delta.x).clamp(left + min_w, viewport.width_px as i32);
            }
            _ => {}
        }
        match edge {
            ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight => {
                top = (top + delta.y).clamp(0, bottom - min_h);
            }
            ResizeEdge::Bottom | ResizeEdge::BottomLeft | ResizeEdge::BottomRight => {
                bottom = (bottom + delta.y).clamp(top + min_h, viewport.height_px as i32);
            }
            _ => {}
        }
        if right - left > max_w {
            if matches!(
                edge,
                ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft
            ) {
                left = right - max_w;
            } else {
                right = left + max_w;
            }
        }
        if bottom - top > max_h {
            if matches!(
                edge,
                ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight
            ) {
                top = bottom - max_h;
            } else {
                bottom = top + max_h;
            }
        }
        let w = (right - left).max(1) as u32;
        let h = (bottom - top).max(1) as u32;
        self.size = clamp_size(size_from_px(w, h, bounds), self.size_min, self.size_max);
        self.position = FreePanelPosition::Offset(
            Percent::from_px(left - bounds.x, bounds.w),
            Percent::from_px(top - bounds.y, bounds.h),
        );
    }
}

/// Identity of a floating slot inside its collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct FloatKey(u64);

impl FloatKey {
    /// Returns the raw identifier used by rendering keys.
    pub(crate) fn raw(self) -> u64 {
        self.0
    }

    /// Rebuilds a key from a rendering identity.
    pub(crate) fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}

/// Why a floating slot was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatError {
    /// The surface already hosts a floating slot with one of these tabs.
    DuplicateTab(TypeId),
}

/// Floating slots in z order, back to front.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FloatPanels {
    panels: IndexMap<FloatKey, FloatPanel>,
    next: u64,
}

impl FloatPanels {
    /// Inserts a floating slot above the others.
    pub fn insert(&mut self, panel: FloatPanel) -> Result<(), FloatError> {
        let incoming = panel.slot.tab_ids();
        if let Some(existing) = self.panels.values().find_map(|entry| {
            entry
                .slot
                .tab_ids()
                .into_iter()
                .find(|id| incoming.contains(id))
        }) {
            return Err(FloatError::DuplicateTab(existing));
        }
        let key = FloatKey(self.next);
        self.next += 1;
        self.panels.insert(key, panel);
        Ok(())
    }

    /// Returns the number of floating slots.
    pub fn len(&self) -> usize {
        self.panels.len()
    }

    /// Returns whether no floating slots are present.
    pub fn is_empty(&self) -> bool {
        self.panels.is_empty()
    }

    /// Iterates floating slots in z order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (FloatKey, &FloatPanel)> + '_ {
        self.panels.iter().map(|(key, panel)| (*key, panel))
    }

    /// Returns a floating slot by identity.
    pub(crate) fn float_by_key(&self, key: FloatKey) -> Option<&FloatPanel> {
        self.panels.get(&key)
    }

    /// Returns a mutable floating slot by identity.
    pub(crate) fn get_mut(&mut self, key: FloatKey) -> Option<&mut FloatPanel> {
        self.panels.get_mut(&key)
    }

    /// Returns the key of the floating slot that hosts a tab.
    pub(crate) fn key_by_tab_id(&self, id: TypeId) -> Option<FloatKey> {
        self.panels
            .iter()
            .find(|(_, panel)| panel.slot.title_bar.contains(id))
            .map(|(key, _)| *key)
    }

    /// Returns the key of the floating slot that hosts a tab type.
    pub(crate) fn key_by_tab<T: Tab>(&self) -> Option<FloatKey> {
        self.key_by_tab_id(TypeId::of::<T>())
    }

    /// Returns the floating slot that hosts a tab.
    pub fn float_by_tab_id(&self, id: TypeId) -> Option<&FloatPanel> {
        self.key_by_tab_id(id).and_then(|key| self.panels.get(&key))
    }

    /// Returns the floating slot that hosts a tab type.
    pub fn float_by_tab<T: Tab>(&self) -> Option<&FloatPanel> {
        self.float_by_tab_id(TypeId::of::<T>())
    }

    /// Raises a floating slot to the top, keeping its key.
    pub(crate) fn raise(&mut self, key: FloatKey) -> bool {
        let Some(panel) = self.panels.shift_remove(&key) else {
            return false;
        };
        self.panels.insert(key, panel);
        true
    }

    /// Removes a floating slot.
    pub(crate) fn close(&mut self, key: FloatKey) -> Option<FloatPanel> {
        self.panels.shift_remove(&key)
    }

    /// Raises the floating slot that hosts a tab type.
    pub fn raise_by_tab<T: Tab>(&mut self) -> bool {
        self.key_by_tab::<T>()
            .map(|key| self.raise(key))
            .unwrap_or(false)
    }

    /// Closes the floating slot that hosts a tab type.
    pub fn close_by_tab<T: Tab>(&mut self) -> Option<FloatPanel> {
        self.key_by_tab::<T>().and_then(|key| self.close(key))
    }

    /// Selects a tab inside its floating slot and raises the slot.
    pub fn select_by_tab(&mut self, id: TypeId) -> bool {
        let Some(key) = self.key_by_tab_id(id) else {
            return false;
        };
        if let Some(panel) = self.panels.get_mut(&key) {
            panel.slot.select_id(id);
        }
        self.raise(key);
        true
    }

    /// Moves a floating slot from a gesture origin.
    pub(crate) fn drag(
        &mut self,
        key: FloatKey,
        bounds: RectPx,
        viewport: Viewport,
        start: RectPx,
        delta: Vec2Px,
    ) {
        if let Some(panel) = self.panels.get_mut(&key) {
            panel.drag(bounds, viewport, start, delta);
        }
    }

    /// Moves a floating slot addressed by a rendering key.
    pub fn drag_raw(
        &mut self,
        raw: u64,
        bounds: RectPx,
        viewport: Viewport,
        start: RectPx,
        delta: Vec2Px,
    ) {
        self.drag(FloatKey(raw), bounds, viewport, start, delta);
    }

    /// Resizes a floating slot from a gesture origin.
    pub(crate) fn resize(
        &mut self,
        key: FloatKey,
        bounds: RectPx,
        viewport: Viewport,
        start: RectPx,
        edge: ResizeEdge,
        delta: Vec2Px,
    ) {
        if let Some(panel) = self.panels.get_mut(&key) {
            panel.resize(bounds, viewport, start, edge, delta);
        }
    }

    /// Resizes a floating slot addressed by a rendering key.
    pub fn resize_raw(
        &mut self,
        raw: u64,
        bounds: RectPx,
        viewport: Viewport,
        start: RectPx,
        edge: ResizeEdge,
        delta: Vec2Px,
    ) {
        self.resize(FloatKey(raw), bounds, viewport, start, edge, delta);
    }
}

impl FromIterator<FloatPanel> for FloatPanels {
    fn from_iter<T: IntoIterator<Item = FloatPanel>>(iter: T) -> Self {
        let mut panels = Self::default();
        for panel in iter {
            let _ = panels.insert(panel);
        }
        panels
    }
}
