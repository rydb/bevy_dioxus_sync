use super::percent::{Percent, PercentSize};
use super::slot::{FiveSlots, SlotSide};

/// Share of an axis an occupied edge slot takes.
pub const EDGE_SHARE: Percent = Percent::new(22);
/// Smallest share an edge slot keeps when resized.
pub const EDGE_MIN_PERCENT: Percent = Percent::new(8);
/// Smallest edge slot size in pixels.
pub const EDGE_MIN_PX: u32 = 48;
/// Largest share an edge slot keeps when resized.
pub const EDGE_MAX_PERCENT: Percent = Percent::new(45);
/// Smallest center region kept when an edge is resized.
pub const CENTER_MIN_PX: u32 = 120;
/// Thickness of an edge grip in pixels.
pub const EDGE_GRIP_PX: u32 = 8;
/// Pixel height of one title bar.
pub const TITLE_BAR_PX: u32 = 24;
/// Percent of a floating slot that must stay visible.
pub const MIN_VISIBLE_PERCENT: u16 = 10;

/// Pixel rectangle used by the projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct RectPx {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl RectPx {
    /// Empty rectangle.
    pub const ZERO: Self = Self {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };

    /// Right edge.
    pub fn right(self) -> i32 {
        self.x.saturating_add(self.w as i32)
    }

    /// Bottom edge.
    pub fn bottom(self) -> i32 {
        self.y.saturating_add(self.h as i32)
    }

    /// Returns whether a point lies inside the rectangle.
    pub fn contains(self, point: Vec2Px) -> bool {
        point.x >= self.x && point.y >= self.y && point.x < self.right() && point.y < self.bottom()
    }
}

/// Pixel vector used by gestures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Vec2Px {
    pub x: i32,
    pub y: i32,
}

/// Surface size in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Viewport {
    pub width_px: u32,
    pub height_px: u32,
}

/// Resize handle of a floating slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResizeEdge {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl ResizeEdge {
    /// Cursor shorthand for the handle.
    pub fn cursor_token(self) -> &'static str {
        match self {
            Self::Left | Self::Right => "ew",
            Self::Top | Self::Bottom => "ns",
            Self::TopLeft | Self::BottomRight => "nwse",
            Self::TopRight | Self::BottomLeft => "nesw",
        }
    }
}

/// Rectangles solved for the five slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotRects {
    pub top: Option<RectPx>,
    pub left: Option<RectPx>,
    pub right: Option<RectPx>,
    pub bottom: Option<RectPx>,
    pub center: RectPx,
}

impl SlotRects {
    /// Returns the rectangle for a side.
    pub fn get(&self, side: SlotSide) -> Option<RectPx> {
        match side {
            SlotSide::Top => self.top,
            SlotSide::Left => self.left,
            SlotSide::Right => self.right,
            SlotSide::Bottom => self.bottom,
            SlotSide::Center => Some(self.center),
        }
    }
}

/// Returns the rectangle each slot occupies for the current occupancy.
pub fn slot_rects(slots: &FiveSlots, viewport: Viewport) -> SlotRects {
    let width = viewport.width_px;
    let height = viewport.height_px;
    let rule = slots.edge_fill_rule;

    // Each occupied edge takes its own share. A short axis squeezes both
    // edges proportionally so the center keeps its minimum when possible.
    let raw_top = if slots.top.is_occupied() {
        slots.edge_sizes.top.as_px(height)
    } else {
        0
    };
    let raw_bottom = if slots.bottom.is_occupied() {
        slots.edge_sizes.bottom.as_px(height)
    } else {
        0
    };
    let (top_h, bottom_h) = fitted_pair(raw_top, raw_bottom, height);

    let raw_left = if slots.left.is_occupied() {
        slots.edge_sizes.left.as_px(width)
    } else {
        0
    };
    let raw_right = if slots.right.is_occupied() {
        slots.edge_sizes.right.as_px(width)
    } else {
        0
    };
    let (left_w, right_w) = fitted_pair(raw_left, raw_right, width);

    let band_x = left_w as i32;
    let band_y = top_h as i32;
    let band_w = width.saturating_sub(left_w).saturating_sub(right_w);
    let band_h = height.saturating_sub(top_h).saturating_sub(bottom_h);

    let (top_x, top_w) = rule.axis_span(SlotSide::Top, width, left_w, right_w);
    let (bottom_x, bottom_w) = rule.axis_span(SlotSide::Bottom, width, left_w, right_w);
    let (left_y, left_h) = rule.axis_span(SlotSide::Left, height, top_h, bottom_h);
    let (right_y, right_h) = rule.axis_span(SlotSide::Right, height, top_h, bottom_h);

    SlotRects {
        top: slots.top.is_occupied().then_some(RectPx {
            x: top_x,
            y: 0,
            w: top_w,
            h: top_h,
        }),
        bottom: slots.bottom.is_occupied().then_some(RectPx {
            x: bottom_x,
            y: height.saturating_sub(bottom_h) as i32,
            w: bottom_w,
            h: bottom_h,
        }),
        left: slots.left.is_occupied().then_some(RectPx {
            x: 0,
            y: left_y,
            w: left_w,
            h: left_h,
        }),
        right: slots.right.is_occupied().then_some(RectPx {
            x: width.saturating_sub(right_w) as i32,
            y: right_y,
            w: right_w,
            h: right_h,
        }),
        center: RectPx {
            x: band_x,
            y: band_y,
            w: band_w,
            h: band_h,
        },
    }
}

/// Fits two edge sizes on one axis while reserving the center minimum.
fn fitted_pair(first: u32, second: u32, axis: u32) -> (u32, u32) {
    let room = if axis > CENTER_MIN_PX {
        axis - CENTER_MIN_PX
    } else {
        axis
    };
    let total = first.saturating_add(second);
    if total <= room || total == 0 {
        return (first.min(axis), second.min(axis));
    }
    let scaled = (first as u64 * room as u64 / total as u64) as u32;
    let first = scaled.min(axis);
    let second = room.saturating_sub(first).min(axis);
    (first, second)
}

/// Returns the minimum and maximum pixel size of one edge.
pub fn edge_bounds(side: SlotSide, slots: &FiveSlots, viewport: Viewport) -> (u32, u32) {
    let (axis, other) = match side {
        SlotSide::Top => (
            viewport.height_px,
            if slots.bottom.is_occupied() {
                slots.edge_sizes.bottom.as_px(viewport.height_px)
            } else {
                0
            },
        ),
        SlotSide::Bottom => (
            viewport.height_px,
            if slots.top.is_occupied() {
                slots.edge_sizes.top.as_px(viewport.height_px)
            } else {
                0
            },
        ),
        SlotSide::Left => (
            viewport.width_px,
            if slots.right.is_occupied() {
                slots.edge_sizes.right.as_px(viewport.width_px)
            } else {
                0
            },
        ),
        SlotSide::Right => (
            viewport.width_px,
            if slots.left.is_occupied() {
                slots.edge_sizes.left.as_px(viewport.width_px)
            } else {
                0
            },
        ),
        SlotSide::Center => (0, 0),
    };
    let min = EDGE_MIN_PERCENT.as_px(axis).max(EDGE_MIN_PX).min(axis);
    let max = EDGE_MAX_PERCENT
        .as_px(axis)
        .min(axis.saturating_sub(other).saturating_sub(CENTER_MIN_PX))
        .max(min)
        .min(axis);
    (min, max)
}

/// Returns the center region left between the edge slots.
pub fn center_rect(slots: &FiveSlots, viewport: Viewport) -> RectPx {
    slot_rects(slots, viewport).center
}

/// Returns the surface region that floats are anchored to.
pub fn float_anchor(viewport: Viewport) -> RectPx {
    RectPx {
        x: 0,
        y: 0,
        w: viewport.width_px,
        h: viewport.height_px,
    }
}

/// Pixel size of a minimized slot handle.
pub const MINIMIZED_HANDLE_PX: u32 = 24;
/// Pixel height of the minimized float bar.
pub const MINIMIZED_BAR_PX: u32 = 28;

/// Returns the restore handle for a minimized slot.
pub fn minimized_handle_rect(side: SlotSide, slots: &FiveSlots, viewport: Viewport) -> RectPx {
    let size = MINIMIZED_HANDLE_PX;
    let center = center_rect(slots, viewport);
    match side {
        SlotSide::Top => RectPx {
            x: (viewport.width_px.saturating_sub(size) / 2) as i32,
            y: 0,
            w: size,
            h: size,
        },
        SlotSide::Bottom => RectPx {
            x: (viewport.width_px.saturating_sub(size) / 2) as i32,
            y: viewport.height_px.saturating_sub(size) as i32,
            w: size,
            h: size,
        },
        SlotSide::Left => RectPx {
            x: 0,
            y: (viewport.height_px.saturating_sub(size) / 2) as i32,
            w: size,
            h: size,
        },
        SlotSide::Right => RectPx {
            x: viewport.width_px.saturating_sub(size) as i32,
            y: (viewport.height_px.saturating_sub(size) / 2) as i32,
            w: size,
            h: size,
        },
        SlotSide::Center => RectPx {
            x: center.x.saturating_add(4),
            y: center.y.saturating_add(4),
            w: size,
            h: size,
        },
    }
}

/// Returns the bar that lists minimized floating slots.
pub fn minimized_bar_rect(slots: &FiveSlots, viewport: Viewport) -> RectPx {
    let center = center_rect(slots, viewport);
    let h = MINIMIZED_BAR_PX.min(center.h);
    RectPx {
        x: center.x,
        y: center.bottom().saturating_sub(h as i32),
        w: center.w,
        h,
    }
}

/// Splits a slot rectangle into title and content.
pub fn split_title(rect: RectPx) -> (RectPx, RectPx) {
    let title_h = TITLE_BAR_PX.min(rect.h);
    (
        RectPx {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: title_h,
        },
        RectPx {
            x: rect.x,
            y: rect.y.saturating_add(title_h as i32),
            w: rect.w,
            h: rect.h.saturating_sub(title_h),
        },
    )
}

/// Resolves a size and percent offset against a bounding rectangle.
pub fn anchored_rect(
    size: PercentSize,
    offset_x: Percent,
    offset_y: Percent,
    collides: bool,
    bounds: RectPx,
    viewport: Viewport,
) -> RectPx {
    let (w, h) = size.as_px(bounds.w, bounds.h);
    let rect = RectPx {
        x: bounds.x.saturating_add(offset_x.as_px(bounds.w) as i32),
        y: bounds.y.saturating_add(offset_y.as_px(bounds.h) as i32),
        w,
        h,
    };
    let rect = if collides {
        clamp_inside(rect, bounds)
    } else {
        clamp_visible(rect, viewport)
    };
    clamp_inside(rect, float_anchor(viewport))
}

/// Clamps a rectangle fully inside another rectangle.
pub fn clamp_inside(rect: RectPx, bounds: RectPx) -> RectPx {
    let w = rect.w.min(bounds.w);
    let h = rect.h.min(bounds.h);
    let max_x = bounds.right().saturating_sub(w as i32);
    let max_y = bounds.bottom().saturating_sub(h as i32);
    RectPx {
        x: rect.x.clamp(bounds.x, max_x.max(bounds.x)),
        y: rect.y.clamp(bounds.y, max_y.max(bounds.y)),
        w,
        h,
    }
}

/// Clamps a rectangle so a minimum area stays inside the viewport.
pub fn clamp_visible(rect: RectPx, viewport: Viewport) -> RectPx {
    let min_w = min_visible(rect.w).min(viewport.width_px) as i32;
    let min_h = min_visible(rect.h).min(viewport.height_px) as i32;
    let max_x = (viewport.width_px as i32).saturating_sub(min_w);
    let min_x = -(rect.w as i32).saturating_add(min_w);
    let max_y = (viewport.height_px as i32).saturating_sub(min_h);
    let min_y = -(rect.h as i32).saturating_add(min_h);
    RectPx {
        x: rect.x.clamp(min_x, max_x),
        y: rect.y.clamp(min_y, max_y),
        ..rect
    }
}

/// Minimum visible extent for one axis.
fn min_visible(extent: u32) -> u32 {
    (extent.saturating_mul(MIN_VISIBLE_PERCENT as u32) / 100).max(1)
}

/// Slides a rectangle by a delta, dropping an axis that would overflow.
pub fn slides(
    rect: RectPx,
    delta: Vec2Px,
    collides: bool,
    bounds: RectPx,
    viewport: Viewport,
) -> RectPx {
    let desired = RectPx {
        x: rect.x.saturating_add(delta.x),
        y: rect.y.saturating_add(delta.y),
        ..rect
    };
    let resolved = if collides {
        clamp_inside(desired, bounds)
    } else {
        clamp_visible(desired, viewport)
    };
    let x = if resolved.x == desired.x {
        desired.x
    } else {
        rect.x
    };
    let y = if resolved.y == desired.y {
        desired.y
    } else {
        rect.y
    };
    RectPx { x, y, ..rect }
}

/// Clamps a size between a minimum and a maximum.
pub fn clamp_size(size: PercentSize, min: PercentSize, max: PercentSize) -> PercentSize {
    PercentSize::new(
        size.width.clamp(min.width, max.width),
        size.height.clamp(min.height, max.height),
    )
}

/// Converts a pixel size to whole percent of a bounding rectangle.
pub fn size_from_px(width_px: u32, height_px: u32, bounds: RectPx) -> PercentSize {
    PercentSize::new(
        percent_of(width_px, bounds.w),
        percent_of(height_px, bounds.h),
    )
}

fn percent_of(px: u32, extent: u32) -> u16 {
    if extent == 0 {
        return 0;
    }
    ((px as u64 * 100 + extent as u64 / 2) / extent as u64).min(100) as u16
}
