use std::any::TypeId;
use std::sync::{Arc, Mutex};

use bevy_window::SystemCursorIcon;
use dioxus::prelude::*;
use dioxus_core::{Element, Event, VNode, use_hook};
use dioxus_core_macro::{Props, component, rsx};
use dioxus_hooks::{use_context_provider, use_future, use_memo, use_signal};
use dioxus_signals::{ReadSignal, ReadableExt, Signal, WritableExt};

use crate::SurfaceCursorHint;
use crate::layout::DioxusPointerReceiver;

use super::CapturedPointer;
use super::command::{
    CommandQueue, CommandReceiver, CommandSender, MoveFloat, OpenSlot, RaiseFloat, ResizeFloat,
    ResizeSlot, SelectTab, TitleOpCommand,
};
use super::float::{FloatKey, FloatPanel};
use super::slot::{EdgeResizeRule, FiveSlots, Slot, SlotSide, SlotState, TitleBar};
use super::solver::{
    EDGE_GRIP_PX, RectPx, ResizeEdge, Vec2Px, Viewport, clamp_inside, float_anchor,
    minimized_bar_rect, minimized_handle_rect, slot_rects, split_title,
};
use super::style::SlotsStyle;
use super::tab::TabFn;
use super::title::TitleOp;

/// Identity used for dioxus keys and pointer routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PanelKey {
    Slot(SlotSide),
    SlotTab(SlotSide, TypeId),
    SlotResize(SlotSide),
    Float(u64),
    FloatTab(u64, TypeId),
    FloatResize(u64, ResizeEdge),
}

/// Geometry of one open slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SlotLayout {
    pub rect: RectPx,
    pub title_rect: RectPx,
    pub content_rect: RectPx,
}

/// Geometry of the five slots.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SlotLayouts {
    pub top: Option<SlotLayout>,
    pub left: Option<SlotLayout>,
    pub right: Option<SlotLayout>,
    pub bottom: Option<SlotLayout>,
    pub center: Option<SlotLayout>,
}

impl SlotLayouts {
    /// Returns the layout for a side.
    pub fn get(&self, side: SlotSide) -> Option<SlotLayout> {
        match side {
            SlotSide::Top => self.top,
            SlotSide::Left => self.left,
            SlotSide::Right => self.right,
            SlotSide::Bottom => self.bottom,
            SlotSide::Center => self.center,
        }
    }

    fn set(&mut self, side: SlotSide, value: Option<SlotLayout>) {
        match side {
            SlotSide::Top => self.top = value,
            SlotSide::Left => self.left = value,
            SlotSide::Right => self.right = value,
            SlotSide::Bottom => self.bottom = value,
            SlotSide::Center => self.center = value,
        }
    }
}

/// Geometry of one open floating slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FloatLayout {
    pub raw: u64,
    pub rect: RectPx,
    pub title_rect: RectPx,
    pub content_rect: RectPx,
    pub z: i32,
}

/// Axis a slot grip resizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GripAxis {
    /// Left and right edges resize horizontally.
    Horizontal,
    /// Top and bottom edges resize vertically.
    Vertical,
}

/// Grab area of one resizable edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotGrip {
    pub side: SlotSide,
    pub rect: RectPx,
    pub axis: GripAxis,
    pub start_px: u32,
}

/// Geometry derived from the model and viewport.
#[derive(Clone, Default, PartialEq)]
pub struct SurfaceLayout {
    pub slots: SlotLayouts,
    pub floats: Vec<FloatLayout>,
    pub slot_grips: Vec<SlotGrip>,
    pub minimized_slots: Vec<(SlotSide, RectPx)>,
    pub minimized_floats: Vec<u64>,
    pub minimized_bar: Option<RectPx>,
}

impl SurfaceLayout {
    /// Returns the restore handle of a minimized slot.
    pub fn minimized_handle(&self, side: SlotSide) -> Option<RectPx> {
        self.minimized_slots
            .iter()
            .find(|(entry, _)| *entry == side)
            .map(|(_, rect)| *rect)
    }
}

/// Derives the surface geometry from the model and viewport.
pub fn layout(slots: &FiveSlots, viewport: Viewport) -> SurfaceLayout {
    let rects = slot_rects(slots, viewport);
    let mut layout = SurfaceLayout::default();

    for side in SlotSide::ALL {
        let slot = slots.slot(side);
        let rule = slots.edge_resize.rule(side);
        match slot.state {
            SlotState::Closed => {}
            SlotState::Minimized => {
                if slot.active_tab.is_none() && !slot.has_tabs() {
                    continue;
                }
                if rule == EdgeResizeRule::Resize {
                    layout.slot_grips.push(SlotGrip {
                        side,
                        rect: edge_restore_grip(side, viewport),
                        axis: grip_axis(side),
                        start_px: 0,
                    });
                } else {
                    layout
                        .minimized_slots
                        .push((side, minimized_handle_rect(side, slots, viewport)));
                }
            }
            SlotState::Open => {
                if slot.active_tab.is_none() && !slot.has_tabs() {
                    continue;
                }
                let Some(rect) = rects.get(side) else {
                    continue;
                };
                let (title_rect, content_rect) = match &slot.title_bar {
                    TitleBar::Enabled { .. } => split_title(rect),
                    TitleBar::Disabled => (RectPx::ZERO, rect),
                };
                layout.slots.set(
                    side,
                    Some(SlotLayout {
                        rect,
                        title_rect,
                        content_rect,
                    }),
                );
                if side != SlotSide::Center {
                    let start_px = match side {
                        SlotSide::Top | SlotSide::Bottom => rect.h,
                        SlotSide::Left | SlotSide::Right => rect.w,
                        SlotSide::Center => 0,
                    };
                    layout.slot_grips.push(SlotGrip {
                        side,
                        rect: edge_grip_rect(side, rect, viewport),
                        axis: grip_axis(side),
                        start_px,
                    });
                }
            }
        }
    }

    let anchor = float_anchor(viewport);
    for (index, (key, panel)) in slots.float_panels.iter().enumerate() {
        let raw = key.raw();
        match panel.slot.state {
            SlotState::Closed => {}
            SlotState::Minimized => layout.minimized_floats.push(raw),
            SlotState::Open => {
                let rect = panel.rect(anchor, viewport);
                let (title_rect, content_rect) = match &panel.slot.title_bar {
                    TitleBar::Enabled { .. } => split_title(rect),
                    TitleBar::Disabled => (RectPx::ZERO, rect),
                };
                layout.floats.push(FloatLayout {
                    raw,
                    rect,
                    title_rect,
                    content_rect,
                    z: 10 + index as i32,
                });
            }
        }
    }

    if !layout.minimized_floats.is_empty() {
        layout.minimized_bar = Some(minimized_bar_rect(slots, viewport));
    }

    layout
}

/// Returns the grip axis for an edge side.
fn grip_axis(side: SlotSide) -> GripAxis {
    match side {
        SlotSide::Left | SlotSide::Right => GripAxis::Horizontal,
        SlotSide::Top | SlotSide::Bottom | SlotSide::Center => GripAxis::Vertical,
    }
}

/// Returns the grip centered on an open edge boundary.
fn edge_grip_rect(side: SlotSide, slot: RectPx, viewport: Viewport) -> RectPx {
    let thickness = EDGE_GRIP_PX;
    let half = (thickness / 2) as i32;
    let rect = match side {
        SlotSide::Left => RectPx {
            x: slot.right().saturating_sub(half),
            y: slot.y,
            w: thickness,
            h: slot.h,
        },
        SlotSide::Right => RectPx {
            x: slot.x.saturating_sub(half),
            y: slot.y,
            w: thickness,
            h: slot.h,
        },
        SlotSide::Top => RectPx {
            x: slot.x,
            y: slot.bottom().saturating_sub(half),
            w: slot.w,
            h: thickness,
        },
        SlotSide::Bottom => RectPx {
            x: slot.x,
            y: slot.y.saturating_sub(half),
            w: slot.w,
            h: thickness,
        },
        SlotSide::Center => RectPx::ZERO,
    };
    clamp_inside(rect, float_anchor(viewport))
}

/// Returns the restore grip along a surface edge.
fn edge_restore_grip(side: SlotSide, viewport: Viewport) -> RectPx {
    let thickness = EDGE_GRIP_PX;
    let width = viewport.width_px;
    let height = viewport.height_px;
    let rect = match side {
        SlotSide::Left => RectPx {
            x: 0,
            y: 0,
            w: thickness,
            h: height,
        },
        SlotSide::Right => RectPx {
            x: width.saturating_sub(thickness) as i32,
            y: 0,
            w: thickness,
            h: height,
        },
        SlotSide::Top => RectPx {
            x: 0,
            y: 0,
            w: width,
            h: thickness,
        },
        SlotSide::Bottom => RectPx {
            x: 0,
            y: height.saturating_sub(thickness) as i32,
            w: width,
            h: thickness,
        },
        SlotSide::Center => RectPx::ZERO,
    };
    clamp_inside(rect, float_anchor(viewport))
}

/// Returns the top most key under a point.
pub fn hit_test(layout: &SurfaceLayout, point: Vec2Px) -> Option<PanelKey> {
    if let Some(float) = layout
        .floats
        .iter()
        .rev()
        .find(|float| float.rect.contains(point))
    {
        return Some(PanelKey::Float(float.raw));
    }
    if let Some(bar) = layout.minimized_bar
        && bar.contains(point)
        && let Some(raw) = layout.minimized_floats.first()
    {
        return Some(PanelKey::Float(*raw));
    }
    if let Some(grip) = layout
        .slot_grips
        .iter()
        .find(|grip| grip.rect.contains(point))
    {
        return Some(PanelKey::SlotResize(grip.side));
    }
    if let Some((side, _)) = layout
        .minimized_slots
        .iter()
        .find(|(_, rect)| rect.contains(point))
    {
        return Some(PanelKey::Slot(*side));
    }
    for side in SlotSide::ALL {
        if let Some(slot) = layout.slots.get(side)
            && slot.rect.contains(point)
        {
            return Some(PanelKey::Slot(side));
        }
    }
    None
}

#[derive(Clone, Copy)]
enum DragKind {
    Move { start: RectPx },
    Resize { edge: ResizeEdge, start: RectPx },
    SlotResize { side: SlotSide, start_px: u32 },
}

#[derive(Clone, Copy)]
struct DragState {
    kind: DragKind,
    target: PanelKey,
    origin: Vec2Px,
}

type CommandChannel = (CommandSender, Arc<Mutex<Option<CommandReceiver>>>);

fn command_channel() -> CommandChannel {
    let (sender, receiver) = CommandReceiver::channel();
    (sender, Arc::new(Mutex::new(Some(receiver))))
}

/// Renders the five slots and the floating collection.
#[component]
pub fn slots_surface(slots: Signal<FiveSlots>, viewport: ReadSignal<Viewport>) -> Element {
    let surface_layout = use_memo(move || layout(&slots.read(), *viewport.read()));
    let drag = use_signal(|| Option::<DragState>::None);
    let (command_sender, receiver_cell) = use_hook(command_channel);
    let pointer_receiver = use_context::<DioxusPointerReceiver>();
    let cursor_hint = use_context::<SurfaceCursorHint>();
    use_context_provider(|| command_sender.clone());

    let mut slots_for_commands = slots;
    let command_receiver_cell = receiver_cell.clone();
    use_future(move || {
        let receiver = command_receiver_cell
            .lock()
            .ok()
            .and_then(|mut guard| guard.take());
        async move {
            let Some(mut receiver) = receiver else {
                return;
            };
            loop {
                let mut queue = CommandQueue::default();
                tokio::select! {
                    Some(job) = receiver.slot_rx.recv() => queue.push_slot_job(job),
                    Some(job) = receiver.surface_rx.recv() => queue.push_surface_job(job),
                    else => break,
                }
                while let Ok(job) = receiver.slot_rx.try_recv() {
                    queue.push_slot_job(job);
                }
                while let Ok(job) = receiver.surface_rx.try_recv() {
                    queue.push_surface_job(job);
                }
                queue.apply(&mut slots_for_commands.write());
            }
        }
    });

    let mut drag_for_capture = drag;
    let command_sender_for_capture = command_sender.clone();
    let command_sender_for_render = command_sender.clone();
    let cursor_hint_for_capture = cursor_hint.clone();
    let viewport_for_capture = viewport;
    let capture_receiver = pointer_receiver.clone();
    use_future(move || {
        let receiver = capture_receiver.clone();
        let sender = command_sender_for_capture.clone();
        let hint = cursor_hint_for_capture.clone();
        async move {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            std::thread::spawn(move || {
                while let Ok(message) = receiver.0.recv() {
                    if tx.send(message).is_err() {
                        break;
                    }
                }
            });
            while let Some(message) = rx.recv().await {
                match message {
                    CapturedPointer::Move(pointer) => {
                        let Some(state) = *drag_for_capture.read() else {
                            continue;
                        };
                        let delta = Vec2Px {
                            x: pointer.x - state.origin.x,
                            y: pointer.y - state.origin.y,
                        };
                        let viewport = *viewport_for_capture.read();
                        let bounds = float_anchor(viewport);
                        match state.kind {
                            DragKind::Move { start } => sender.send_surface(
                                state.target,
                                MoveFloat {
                                    bounds,
                                    viewport,
                                    start,
                                    delta,
                                },
                            ),
                            DragKind::Resize { edge, start } => sender.send_surface(
                                state.target,
                                ResizeFloat {
                                    bounds,
                                    viewport,
                                    start,
                                    edge,
                                    delta,
                                },
                            ),
                            DragKind::SlotResize { side, start_px } => {
                                let delta_px = match side {
                                    SlotSide::Left => delta.x,
                                    SlotSide::Right => -delta.x,
                                    SlotSide::Top => delta.y,
                                    SlotSide::Bottom => -delta.y,
                                    SlotSide::Center => 0,
                                };
                                sender.send_surface(
                                    state.target,
                                    ResizeSlot {
                                        side,
                                        viewport,
                                        start_px,
                                        delta_px,
                                    },
                                );
                            }
                        }
                    }
                    CapturedPointer::Up => {
                        drag_for_capture.set(None);
                        let _ = hint.0.send(None);
                    }
                }
            }
        }
    });

    rsx! {
        div {
            class: "slot-surface",
            position: "absolute",
            left: "0",
            top: "0",
            width: "100%",
            height: "100%",
            overflow: "hidden",
            for side in SlotSide::ALL {
                {render_slot(
                    side,
                    slots.read().slot(side),
                    surface_layout.read().slots.get(side),
                    surface_layout.read().minimized_handle(side),
                    command_sender_for_render.clone(),
                )}
            }
            for float in surface_layout.read().floats.iter().copied() {
                {render_float(
                    float,
                    slots
                        .read()
                        .float_panels
                        .float_by_key(FloatKey::from_raw(float.raw)),
                    command_sender_for_render.clone(),
                    drag,
                )}
            }
            if let Some(bar) = surface_layout.read().minimized_bar {
                {
                    let layout = surface_layout.read();
                    let model = slots.read();
                    render_minimized_bar(bar, &layout.minimized_floats, &model, command_sender_for_render)
                }
            }
            for grip in surface_layout.read().slot_grips.iter().copied() {
                {
                    let active = drag.read().as_ref().is_some_and(|state| {
                        matches!(state.kind, DragKind::SlotResize { side, .. } if side == grip.side)
                    });
                    render_slot_grip(grip, active, drag, cursor_hint.clone())
                }
            }
        }
    }
}

fn render_slot_grip(
    grip: SlotGrip,
    active: bool,
    mut drag: Signal<Option<DragState>>,
    cursor_hint: SurfaceCursorHint,
) -> Element {
    let side = grip.side;
    let start_px = grip.start_px;
    let rect = grip.rect;
    let cursor = match grip.axis {
        GripAxis::Horizontal => "ew",
        GripAxis::Vertical => "ns",
    };
    let key = format!("grip-{:?}", side);
    let class = if active {
        format!(
            "catch-events slot-grip {} {}",
            SlotsStyle::slot_grip,
            SlotsStyle::slot_grip_active
        )
    } else {
        format!("catch-events slot-grip {}", SlotsStyle::slot_grip)
    };
    rsx! {
        div {
            key: "{key}",
            class: "{class}",
            position: "absolute",
            left: "{rect.x}px",
            top: "{rect.y}px",
            width: "{rect.w}px",
            height: "{rect.h}px",
            z_index: 5,
            cursor: "{cursor}",
            pointer_events: "auto",
            onpointerdown: move |event| {
                event.stop_propagation();
                let coords = event.client_coordinates();
                let system_cursor = match grip.axis {
                    GripAxis::Horizontal => SystemCursorIcon::EwResize,
                    GripAxis::Vertical => SystemCursorIcon::NsResize,
                };
                let _ = cursor_hint.0.send(Some(system_cursor));
                drag.set(Some(DragState {
                    kind: DragKind::SlotResize { side, start_px },
                    target: PanelKey::SlotResize(side),
                    origin: pointer_of(coords.x, coords.y),
                }));
            },
        }
    }
}

fn render_slot(
    side: SlotSide,
    slot: &Slot,
    geometry: Option<SlotLayout>,
    handle: Option<RectPx>,
    sender: CommandSender,
) -> Element {
    match slot.state {
        SlotState::Closed => rsx! {},
        SlotState::Minimized => {
            let key = format!("{:?}", PanelKey::Slot(side));
            let hidden = render_hidden_tabs(slot);
            let handle = handle.map(|rect| render_slot_handle(side, rect, sender));
            rsx! {
                div {
                    key: "{key}",
                    display: "none",
                    {hidden}
                }
                for element in handle {
                    {element}
                }
            }
        }
        SlotState::Open => {
            let Some(geometry) = geometry else {
                return rsx! {};
            };
            let key = format!("{:?}", PanelKey::Slot(side));
            let rect = geometry.rect;
            let title = geometry.title_rect;
            let body = geometry.content_rect;
            let target = PanelKey::Slot(side);
            let tabs = render_tabs(slot, side, None, target, sender.clone());
            let ops = render_ops(slot, target, sender.clone());
            let content = render_visible_tabs(slot);
            rsx! {
                div {
                    key: "{key}",
                    class: "catch-events slot-panel",
                    position: "absolute",
                    left: "{rect.x}px",
                    top: "{rect.y}px",
                    width: "{rect.w}px",
                    height: "{rect.h}px",
                    z_index: 1,
                    box_sizing: "border-box",
                    div {
                        class: "slot-title {SlotsStyle::slot_title}",
                        position: "absolute",
                        left: "0",
                        top: "0",
                        width: "{title.w}px",
                        height: "{title.h}px",
                        z_index: 2,
                        box_sizing: "border-box",
                        {tabs}
                        {ops}
                    }
                    div {
                        class: "slot-body",
                        position: "absolute",
                        left: "0",
                        top: "{title.h}px",
                        width: "{body.w}px",
                        height: "{body.h}px",
                        z_index: 2,
                        box_sizing: "border-box",
                        overflow: "hidden",
                        {content}
                    }
                }
            }
        }
    }
}

fn render_float(
    geometry: FloatLayout,
    panel: Option<&FloatPanel>,
    sender: CommandSender,
    mut drag: Signal<Option<DragState>>,
) -> Element {
    let Some(panel) = panel else {
        return rsx! {};
    };
    let slot = &panel.slot;
    let target = PanelKey::Float(geometry.raw);
    let key = format!("{target:?}");
    let rect = geometry.rect;
    let title = geometry.title_rect;
    let body = geometry.content_rect;
    let z = geometry.z;
    let inner_z = z + 1;
    let handles = if panel.resizable {
        resize_handles(RectPx {
            x: 0,
            y: 0,
            ..geometry.rect
        })
    } else {
        Vec::new()
    };
    let movable = matches!(slot.title_bar, TitleBar::Enabled { .. });
    let tabs = render_tabs(
        slot,
        SlotSide::Center,
        Some(geometry.raw),
        target,
        sender.clone(),
    );
    let ops = render_ops(slot, target, sender.clone());
    let content = render_visible_tabs(slot);
    let sender_for_title = sender.clone();

    let onpointerdown = move |event: Event<PointerData>| {
        if !movable {
            return;
        }
        event.stop_propagation();
        let coords = event.client_coordinates();
        sender_for_title.send_surface(target, RaiseFloat);
        drag.set(Some(DragState {
            kind: DragKind::Move {
                start: geometry.rect,
            },
            target,
            origin: pointer_of(coords.x, coords.y),
        }));
    };

    rsx! {
        div {
            key: "{key}",
            class: "catch-events slot-panel slot-float",
            position: "absolute",
            left: "{rect.x}px",
            top: "{rect.y}px",
            width: "{rect.w}px",
            height: "{rect.h}px",
            z_index: z,
            box_sizing: "border-box",
            div {
                class: "slot-title {SlotsStyle::slot_title}",
                position: "absolute",
                left: "0",
                top: "0",
                width: "{title.w}px",
                height: "{title.h}px",
                z_index: inner_z,
                box_sizing: "border-box",
                onpointerdown,
                {tabs}
                {ops}
            }
            div {
                class: "slot-body",
                position: "absolute",
                left: "0",
                top: "{title.h}px",
                width: "{body.w}px",
                height: "{body.h}px",
                z_index: inner_z,
                box_sizing: "border-box",
                overflow: "hidden",
                {content}
            }
            for (edge, rect) in handles {
                {render_resize_handle(target, edge, geometry.rect, rect, drag)}
            }
        }
    }
}

fn render_slot_handle(side: SlotSide, rect: RectPx, sender: CommandSender) -> Element {
    let key = format!("{:?}", PanelKey::Slot(side));
    rsx! {
        div {
            key: "minimized-{key}",
            class: "catch-events slot-slot-handle {SlotsStyle::slot_slot_handle}",
            position: "absolute",
            left: "{rect.x}px",
            top: "{rect.y}px",
            width: "{rect.w}px",
            height: "{rect.h}px",
            z_index: 20,
            box_sizing: "border-box",
            onpointerdown: move |event| {
                event.stop_propagation();
                sender.send_slot(PanelKey::Slot(side), OpenSlot);
            },
            "+"
        }
    }
}

fn render_minimized_bar(
    rect: RectPx,
    raws: &[u64],
    model: &FiveSlots,
    sender: CommandSender,
) -> Element {
    let entries = raws
        .iter()
        .enumerate()
        .filter_map(|(index, raw)| {
            let panel = model.float_panels.float_by_key(FloatKey::from_raw(*raw))?;
            let title = slot_title(&panel.slot);
            let title = if title.is_empty() {
                format!("Float {}", index + 1)
            } else {
                title
            };
            Some(render_minimized_entry(*raw, title, sender.clone()))
        })
        .collect::<Vec<_>>();
    rsx! {
        div {
            class: "catch-events slot-minimized-bar {SlotsStyle::slot_minimized_bar}",
            position: "absolute",
            left: "{rect.x}px",
            top: "{rect.y}px",
            width: "{rect.w}px",
            height: "{rect.h}px",
            z_index: 20,
            box_sizing: "border-box",
            overflow: "hidden",
            for entry in entries {
                {entry}
            }
        }
    }
}

fn render_minimized_entry(raw: u64, title: String, sender: CommandSender) -> Element {
    let key = format!("minimized-{raw}");
    rsx! {
        button {
            key: "{key}",
            class: "slot-minimized-entry {SlotsStyle::slot_minimized_entry}",
            flex_shrink: 0,
            onpointerdown: move |event| {
                event.stop_propagation();
                sender.send_slot(PanelKey::Float(raw), OpenSlot);
            },
            "{title}"
        }
    }
}

fn render_tabs(
    slot: &Slot,
    side: SlotSide,
    float: Option<u64>,
    target: PanelKey,
    sender: CommandSender,
) -> Element {
    let tabs = tab_renders(slot, side, float);
    rsx! {
        for tab in tabs {
            {render_tab(tab, target, sender.clone())}
        }
    }
}

fn render_tab(tab: TabRender, target: PanelKey, sender: CommandSender) -> Element {
    let key = format!("{:?}", tab.key);
    let class = if tab.active {
        "slot-tab-active"
    } else {
        "slot-tab"
    };
    let id = tab.id;
    rsx! {
        button {
            key: "{key}",
            class: "{class}",
            onpointerdown: move |event| {
                event.stop_propagation();
                sender.send_slot(target, SelectTab(id));
            },
            "{tab.title}"
        }
    }
}

fn render_ops(slot: &Slot, target: PanelKey, sender: CommandSender) -> Element {
    let ops = slot
        .title_bar
        .ops()
        .map(|ops| ops.iter().map(|(_, op)| op.clone()).collect::<Vec<_>>())
        .unwrap_or_default();
    rsx! {
        div {
            margin_left: "auto",
            display: "flex",
            align_items: "center",
            gap: "4px",
            for op in ops {
                {render_op(op, target, sender.clone())}
            }
        }
    }
}

fn render_op(op: Arc<dyn TitleOp>, target: PanelKey, sender: CommandSender) -> Element {
    let icon = op.icon();
    let key = format!("op-{:p}", Arc::as_ptr(&op));
    rsx! {
        button {
            key: "{key}",
            class: "slot-op {SlotsStyle::slot_op}",
            onpointerdown: move |event| {
                event.stop_propagation();
                match op.command() {
                    TitleOpCommand::Slot(command) => sender.send_slot_boxed(target, command),
                    TitleOpCommand::Surface(command) => sender.send_surface_boxed(target, command),
                }
            },
            {icon}
        }
    }
}

/// Content renderer for one tab body.
#[derive(Clone, Copy, Debug)]
struct TabView {
    id: TypeId,
    render: TabFn,
}

impl PartialEq for TabView {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// Hosts one tab body so its hooks belong to the tab rather than the surface.
#[component]
fn tab_body(view: TabView) -> Element {
    (view.render)()
}

fn render_visible_tabs(slot: &Slot) -> Element {
    let active = slot.active_id();
    let group = slot.title_bar.group();
    let elements = slot
        .mounted_tabs()
        .into_iter()
        .map(|id| {
            let element = group
                .and_then(|group| group.element(id))
                .unwrap_or(empty_element);
            (id, active == Some(id), element)
        })
        .collect::<Vec<_>>();
    rsx! {
        for (id, is_active, element) in elements {
            div {
                key: "body-{id:?}",
                display: if is_active { "block" } else { "none" },
                width: "100%",
                height: "100%",
                min_width: "0",
                min_height: "0",
                overflow: "auto",
                box_sizing: "border-box",
                tab_body { view: TabView { id, render: element } }
            }
        }
    }
}

fn render_hidden_tabs(slot: &Slot) -> Element {
    let group = slot.title_bar.group();
    let elements = slot
        .mounted_tabs()
        .into_iter()
        .map(|id| {
            let element = group
                .and_then(|group| group.element(id))
                .unwrap_or(empty_element);
            (id, element)
        })
        .collect::<Vec<_>>();
    rsx! {
        for (id, element) in elements {
            div {
                key: "body-{id:?}",
                tab_body { view: TabView { id, render: element } }
            }
        }
    }
}

fn render_resize_handle(
    target: PanelKey,
    edge: ResizeEdge,
    outer: RectPx,
    rect: RectPx,
    mut drag: Signal<Option<DragState>>,
) -> Element {
    let cursor = edge.cursor_token();
    let key = format!("{:?}-{cursor}", rect.x);
    rsx! {
        div {
            key: "resize-{key}",
            position: "absolute",
            left: "{rect.x}px",
            top: "{rect.y}px",
            width: "{rect.w}px",
            height: "{rect.h}px",
            cursor: "{cursor}",
            pointer_events: "auto",
            onpointerdown: move |event| {
                event.stop_propagation();
                let coords = event.client_coordinates();
                drag.set(Some(DragState {
                    kind: DragKind::Resize { edge, start: outer },
                    target,
                    origin: pointer_of(coords.x, coords.y),
                }));
            },
        }
    }
}

fn resize_handles(outer: RectPx) -> Vec<(ResizeEdge, RectPx)> {
    let edge = 6.min(outer.w).min(outer.h);
    let corner = 12.min(outer.w).min(outer.h);
    let horizontal = outer.w.saturating_sub(corner.saturating_mul(2));
    let vertical = outer.h.saturating_sub(corner.saturating_mul(2));
    vec![
        (
            ResizeEdge::TopLeft,
            RectPx {
                x: outer.x,
                y: outer.y,
                w: corner,
                h: corner,
            },
        ),
        (
            ResizeEdge::TopRight,
            RectPx {
                x: outer.right().saturating_sub(corner as i32),
                y: outer.y,
                w: corner,
                h: corner,
            },
        ),
        (
            ResizeEdge::BottomLeft,
            RectPx {
                x: outer.x,
                y: outer.bottom().saturating_sub(corner as i32),
                w: corner,
                h: corner,
            },
        ),
        (
            ResizeEdge::BottomRight,
            RectPx {
                x: outer.right().saturating_sub(corner as i32),
                y: outer.bottom().saturating_sub(corner as i32),
                w: corner,
                h: corner,
            },
        ),
        (
            ResizeEdge::Left,
            RectPx {
                x: outer.x,
                y: outer.y.saturating_add(corner as i32),
                w: edge,
                h: vertical,
            },
        ),
        (
            ResizeEdge::Right,
            RectPx {
                x: outer.right().saturating_sub(edge as i32),
                y: outer.y.saturating_add(corner as i32),
                w: edge,
                h: vertical,
            },
        ),
        (
            ResizeEdge::Top,
            RectPx {
                x: outer.x.saturating_add(corner as i32),
                y: outer.y,
                w: horizontal,
                h: edge,
            },
        ),
        (
            ResizeEdge::Bottom,
            RectPx {
                x: outer.x.saturating_add(corner as i32),
                y: outer.bottom().saturating_sub(edge as i32),
                w: horizontal,
                h: edge,
            },
        ),
    ]
}

#[derive(Clone)]
struct TabRender {
    key: PanelKey,
    id: TypeId,
    title: &'static str,
    active: bool,
}

fn tab_renders(slot: &Slot, side: SlotSide, float: Option<u64>) -> Vec<TabRender> {
    let active = slot.active_id();
    let group = slot.title_bar.group();
    slot.tab_ids()
        .into_iter()
        .map(|id| TabRender {
            key: match float {
                Some(raw) => PanelKey::FloatTab(raw, id),
                None => PanelKey::SlotTab(side, id),
            },
            id,
            title: group.and_then(|group| group.title(id)).unwrap_or(""),
            active: active == Some(id),
        })
        .collect()
}

fn slot_title(slot: &Slot) -> String {
    let group = slot.title_bar.group();
    slot.active_id()
        .or_else(|| slot.tab_ids().first().copied())
        .and_then(|id| group.and_then(|group| group.title(id)))
        .unwrap_or("")
        .to_string()
}

fn empty_element() -> Element {
    Ok(VNode::placeholder())
}

fn pointer_of(x: f64, y: f64) -> Vec2Px {
    Vec2Px {
        x: x.round() as i32,
        y: y.round() as i32,
    }
}
