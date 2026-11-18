//! Slot surface state, commands, and rendering.

mod command;
mod float;
mod percent;
mod slot;
mod solver;
mod style;
mod surface;
mod tab;
mod theme;
mod title;

#[cfg(test)]
mod tests;

pub use bevy_dioxus_slots_macro::{Tab, theme};
pub use command::{
    CloseSlot, CommandQueue, CommandReceiver, CommandSender, ErasedSlotCommand,
    ErasedSurfaceCommand, MinimizeSlot, MoveFloat, OpenSlot, RaiseFloat, ResizeFloat, ResizeSlot,
    SelectTab, SlotCommand, SlotContext, SurfaceCommand, SurfaceContext, TitleOpCommand,
};
pub use float::{FloatError, FloatPanel, FloatPanels, FreePanelPosition};
pub use percent::{Percent, PercentSize};
pub use slot::{
    EdgeFillRule, EdgeResize, EdgeResizeRule, EdgeSizes, FiveSlots, Slot, SlotSide, SlotState,
    TitleBar,
};
pub use solver::{
    CENTER_MIN_PX, EDGE_GRIP_PX, EDGE_MAX_PERCENT, EDGE_MIN_PERCENT, EDGE_MIN_PX, EDGE_SHARE,
    MIN_VISIBLE_PERCENT, RectPx, ResizeEdge, SlotRects, TITLE_BAR_PX, Vec2Px, Viewport,
    anchored_rect, center_rect, clamp_inside, clamp_size, clamp_visible, edge_bounds, float_anchor,
    size_from_px, slides, slot_rects, split_title,
};
pub use surface::{
    FloatLayout, GripAxis, PanelKey, SlotGrip, SlotLayout, SlotLayouts, SurfaceLayout, hit_test,
    layout, slots_surface,
};
pub use tab::{ActiveTab, Name, Tab, TabEntry, TabFn, TabGroup, TabMount, tab_entry};
pub use theme::{DARK, LIGHT, SlotTheme, Theme, css_has_rule};
pub use title::{Close, Minimize, TitleOp, TitleOps};

/// Pointer capture message delivered to a surface outside the DOM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapturedPointer {
    /// Pointer moved to a surface pixel position.
    Move(Vec2Px),
    /// Every pointer button was released.
    Up,
}

/// Builds an enabled title bar from tab types and action values.
#[macro_export]
macro_rules! title_bar {
    ([$($tab:ty),* $(,)?], [$($op:expr),* $(,)?]) => {
        $crate::slots::TitleBar::Enabled {
            group: $crate::slots::TabGroup::from_entries(
                [ $($crate::slots::tab_entry::<$tab>()),* ],
                $crate::slots::TabMount::LazyKeepAlive,
            ),
            ops: $crate::slots::TitleOps::from_ops(
                [ $(::std::sync::Arc::new($op) as ::std::sync::Arc<dyn $crate::slots::TitleOp>),* ],
            ),
        }
    };
}

/// Builds a disabled title bar.
#[macro_export]
macro_rules! title_bar_disabled {
    () => {
        $crate::slots::TitleBar::Disabled
    };
}

/// Builds the active tab for a tab type.
#[macro_export]
macro_rules! active_tab {
    ($tab:ty) => {
        $crate::slots::ActiveTab::new::<$tab>()
    };
}
