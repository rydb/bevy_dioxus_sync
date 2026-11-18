//! Conformance tests for the panel surface state.

use std::any::TypeId;

use dioxus_core::{Element, VNode};

use super::float::FloatKey;
use super::*;

fn body() -> Element {
    Ok(VNode::placeholder())
}

#[derive(Name)]
struct Alpha;
#[derive(Name)]
struct Beta;

impl Tab for Alpha {
    fn element() -> TabFn {
        body
    }
}

impl Tab for Beta {
    fn element() -> TabFn {
        body
    }
}

fn group(mount: TabMount) -> TabGroup {
    TabGroup::from_entries([tab_entry::<Alpha>(), tab_entry::<Beta>()], mount)
}

fn open_slot<T: Tab>(mount: TabMount) -> Slot {
    Slot {
        title_bar: TitleBar::Enabled {
            group: group(mount),
            ops: TitleOps::empty(),
        },
        active_tab: Some(ActiveTab::new::<T>()),
        state: SlotState::Open,
    }
}

fn float_for<T: Tab>() -> FloatPanel {
    FloatPanel {
        slot: Slot {
            title_bar: TitleBar::Enabled {
                group: TabGroup::from_entries([tab_entry::<T>()], TabMount::Eager),
                ops: TitleOps::empty(),
            },
            active_tab: Some(ActiveTab::new::<T>()),
            state: SlotState::Open,
        },
        ..FloatPanel::default()
    }
}

fn viewport() -> Viewport {
    Viewport {
        width_px: 1_000,
        height_px: 500,
    }
}

fn center() -> RectPx {
    RectPx {
        x: 100,
        y: 50,
        w: 800,
        h: 400,
    }
}

const RULES: [EdgeFillRule; 6] = [
    EdgeFillRule::LeftRight,
    EdgeFillRule::TopBottom,
    EdgeFillRule::TopRight,
    EdgeFillRule::TopLeft,
    EdgeFillRule::BottomRight,
    EdgeFillRule::BottomLeft,
];

fn all_edges_open(rule: EdgeFillRule) -> FiveSlots {
    let mut slots = FiveSlots {
        edge_fill_rule: rule,
        ..FiveSlots::default()
    };
    slots.top = open_slot::<Alpha>(TabMount::Eager);
    slots.bottom = open_slot::<Alpha>(TabMount::Eager);
    slots.left = open_slot::<Beta>(TabMount::Eager);
    slots.right = open_slot::<Beta>(TabMount::Eager);
    slots.center = open_slot::<Beta>(TabMount::Eager);
    slots
}

fn open_rects(slots: &FiveSlots, viewport: Viewport) -> Vec<RectPx> {
    let rects = slot_rects(slots, viewport);
    SlotSide::ALL
        .into_iter()
        .filter_map(|side| rects.get(side))
        .collect()
}

fn overlaps(a: RectPx, b: RectPx) -> bool {
    a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
}

fn assert_slots_are_apart(slots: &FiveSlots, viewport: Viewport, rule: EdgeFillRule) {
    let rects = open_rects(slots, viewport);
    assert_eq!(rects.len(), 5, "rule {rule:?}");
    for rect in &rects {
        assert!(rect.x >= 0 && rect.y >= 0, "rule {rule:?} {rect:?}");
        assert!(
            rect.right() <= viewport.width_px as i32,
            "rule {rule:?} {rect:?}"
        );
        assert!(
            rect.bottom() <= viewport.height_px as i32,
            "rule {rule:?} {rect:?}"
        );
    }
    for (index, first) in rects.iter().enumerate() {
        for second in rects.iter().skip(index + 1) {
            assert!(
                !overlaps(*first, *second),
                "rule {rule:?} {first:?} {second:?}"
            );
        }
    }
}

#[test]
fn title_bar_macro_builds_tabs_and_ops() {
    let bar = crate::title_bar!([Alpha, Beta], [Minimize, Close]);
    let group = bar.group().unwrap();
    assert!(group.contains_type::<Alpha>());
    assert!(group.contains_type::<Beta>());
    assert_eq!(group.title(TypeId::of::<Alpha>()), Some("Alpha"));
    assert_eq!(group.title(TypeId::of::<Beta>()), Some("Beta"));
    let ops = bar.ops().unwrap();
    assert!(ops.contains::<Minimize>());
    assert!(ops.contains::<Close>());
    assert_eq!(ops.len(), 2);
}

#[test]
fn select_tab_command_updates_the_active_tab() {
    let mut slots = FiveSlots::default();
    slots.left = open_slot::<Alpha>(TabMount::Eager);
    let mut queue = CommandQueue::default();
    queue.push_slot(
        PanelKey::Slot(SlotSide::Left),
        SelectTab(TypeId::of::<Beta>()),
    );
    queue.apply(&mut slots);
    assert_eq!(slots.left.active_id(), Some(TypeId::of::<Beta>()));
}

#[test]
fn minimising_keeps_the_active_tab_and_group() {
    let mut slots = FiveSlots::default();
    slots.left = open_slot::<Alpha>(TabMount::Eager);
    let tabs = slots.left.tab_ids();
    let mut queue = CommandQueue::default();
    queue.push_slot(PanelKey::Slot(SlotSide::Left), MinimizeSlot);
    queue.apply(&mut slots);
    assert_eq!(slots.left.state, SlotState::Minimized);
    assert_eq!(slots.left.active_id(), Some(TypeId::of::<Alpha>()));
    assert_eq!(slots.left.tab_ids(), tabs);
}

#[test]
fn opening_restores_without_changing_the_active_tab() {
    let mut slots = FiveSlots::default();
    slots.left = open_slot::<Beta>(TabMount::Eager);
    slots.left.state = SlotState::Minimized;
    let mut queue = CommandQueue::default();
    queue.push_slot(PanelKey::Slot(SlotSide::Left), OpenSlot);
    queue.apply(&mut slots);
    assert_eq!(slots.left.state, SlotState::Open);
    assert_eq!(slots.left.active_id(), Some(TypeId::of::<Beta>()));
}

#[test]
fn closing_keeps_the_active_tab_and_group() {
    let mut slots = FiveSlots::default();
    slots.left = open_slot::<Alpha>(TabMount::Eager);
    let tabs = slots.left.tab_ids();
    let mut queue = CommandQueue::default();
    queue.push_surface(PanelKey::Slot(SlotSide::Left), CloseSlot);
    queue.apply(&mut slots);
    assert_eq!(slots.left.state, SlotState::Closed);
    assert_eq!(slots.left.active_id(), Some(TypeId::of::<Alpha>()));
    assert_eq!(slots.left.tab_ids(), tabs);
}

#[test]
fn selecting_does_not_change_slot_state() {
    let mut slots = FiveSlots::default();
    slots.left = open_slot::<Alpha>(TabMount::Eager);
    slots.left.state = SlotState::Minimized;
    let mut queue = CommandQueue::default();
    queue.push_slot(
        PanelKey::Slot(SlotSide::Left),
        SelectTab(TypeId::of::<Beta>()),
    );
    queue.apply(&mut slots);
    assert_eq!(slots.left.state, SlotState::Minimized);
    assert_eq!(slots.left.active_id(), Some(TypeId::of::<Beta>()));
}

#[test]
fn float_minimise_keeps_open_and_close_remove() {
    let mut slots = FiveSlots::default();
    slots.float_panels.insert(float_for::<Alpha>()).unwrap();
    let key = slots.float_panels.key_by_tab::<Alpha>().unwrap();
    let target = PanelKey::Float(key.raw());

    let mut queue = CommandQueue::default();
    queue.push_slot(target, MinimizeSlot);
    queue.apply(&mut slots);
    assert_eq!(
        slots.float_panels.float_by_key(key).unwrap().slot.state,
        SlotState::Minimized
    );

    let mut queue = CommandQueue::default();
    queue.push_slot(target, OpenSlot);
    queue.apply(&mut slots);
    assert_eq!(
        slots.float_panels.float_by_key(key).unwrap().slot.state,
        SlotState::Open
    );

    let mut queue = CommandQueue::default();
    queue.push_surface(target, CloseSlot);
    queue.apply(&mut slots);
    assert!(slots.float_panels.is_empty());
}

#[test]
fn layout_only_includes_open_slots_and_minimised_handles() {
    let mut slots = FiveSlots::default();
    slots.left = open_slot::<Alpha>(TabMount::Eager);
    let open_layout = layout(&slots, viewport());
    assert!(open_layout.slots.left.is_some());
    assert!(open_layout.minimized_slots.is_empty());
    assert!(
        open_layout
            .slot_grips
            .iter()
            .any(|grip| grip.side == SlotSide::Left)
    );

    slots.left.state = SlotState::Minimized;
    let minimized_layout = layout(&slots, viewport());
    assert!(minimized_layout.slots.left.is_none());
    assert!(minimized_layout.minimized_handle(SlotSide::Left).is_none());
    assert!(
        minimized_layout
            .slot_grips
            .iter()
            .any(|grip| grip.side == SlotSide::Left && grip.start_px == 0)
    );

    slots.edge_resize.left = EdgeResizeRule::ResizeNoMinimize;
    let handle_layout = layout(&slots, viewport());
    assert!(handle_layout.minimized_handle(SlotSide::Left).is_some());
    assert!(
        !handle_layout
            .slot_grips
            .iter()
            .any(|grip| grip.side == SlotSide::Left)
    );

    slots.left.state = SlotState::Closed;
    let closed_layout = layout(&slots, viewport());
    assert!(closed_layout.slots.left.is_none());
    assert!(closed_layout.minimized_slots.is_empty());
}

#[test]
fn edge_resize_updates_the_left_share() {
    let viewport = viewport();
    let mut slots = FiveSlots::default();
    slots.left = open_slot::<Alpha>(TabMount::Eager);
    slots.right = open_slot::<Beta>(TabMount::Eager);
    let start = slot_rects(&slots, viewport).left.unwrap().w;

    slots.resize_edge(SlotSide::Left, viewport, start, 100);

    let rects = slot_rects(&slots, viewport);
    assert_eq!(rects.left.unwrap().w, start + 100);
    assert_eq!(
        rects.center.w,
        1_000 - (start + 100) - rects.right.unwrap().w
    );
}

#[test]
fn edge_resize_clamps_to_the_bounds() {
    let viewport = viewport();
    let mut slots = FiveSlots::default();
    slots.left = open_slot::<Alpha>(TabMount::Eager);
    slots.right = open_slot::<Beta>(TabMount::Eager);

    slots.resize_edge(SlotSide::Left, viewport, 220, -10_000);
    assert_eq!(slots.left.state, SlotState::Minimized);

    slots.resize_edge(SlotSide::Left, viewport, 0, 10_000);
    assert_eq!(slots.left.state, SlotState::Open);
    let rects = slot_rects(&slots, viewport);
    assert_eq!(rects.left.unwrap().w, 450);
    assert_eq!(rects.center.w, 1_000 - 450 - rects.right.unwrap().w);
}

#[test]
fn resize_no_minimize_stops_at_the_minimum() {
    let viewport = viewport();
    let mut slots = FiveSlots::default();
    slots.edge_resize.left = EdgeResizeRule::ResizeNoMinimize;
    slots.left = open_slot::<Alpha>(TabMount::Eager);

    slots.resize_edge(SlotSide::Left, viewport, 220, -10_000);

    assert_eq!(slots.left.state, SlotState::Open);
    assert_eq!(slots.edge_sizes.left, Percent::new(8));
    assert_eq!(slot_rects(&slots, viewport).left.unwrap().w, 80);
}

#[test]
fn resize_no_minimize_does_not_restore() {
    let viewport = viewport();
    let mut slots = FiveSlots::default();
    slots.edge_resize.left = EdgeResizeRule::ResizeNoMinimize;
    slots.left = open_slot::<Alpha>(TabMount::Eager);
    slots.left.state = SlotState::Minimized;

    slots.resize_edge(SlotSide::Left, viewport, 0, 400);

    assert_eq!(slots.left.state, SlotState::Minimized);
}

#[test]
fn edge_restore_opens_from_the_surface_edge() {
    let viewport = viewport();
    let mut slots = FiveSlots::default();
    slots.left = open_slot::<Alpha>(TabMount::Eager);
    slots.left.state = SlotState::Minimized;

    slots.resize_edge(SlotSide::Left, viewport, 0, 100);

    assert_eq!(slots.left.state, SlotState::Open);
    assert_eq!(slot_rects(&slots, viewport).left.unwrap().w, 100);
}

#[test]
fn edge_bounds_keep_the_center_free() {
    let viewport = viewport();
    let mut slots = FiveSlots::default();
    slots.top = open_slot::<Alpha>(TabMount::Eager);
    slots.bottom = open_slot::<Beta>(TabMount::Eager);
    slots.edge_sizes.top = Percent::new(45);
    slots.edge_sizes.bottom = Percent::new(45);

    let (_, max) = edge_bounds(SlotSide::Bottom, &slots, viewport);
    assert_eq!(max, 500 - 225 - 120);

    let rects = slot_rects(&slots, viewport);
    assert_eq!(rects.top.unwrap().h, 190);
    assert_eq!(rects.bottom.unwrap().h, 190);
    assert_eq!(rects.center.h, 120);
    assert_eq!(rects.bottom.unwrap().y, 310);
}

#[test]
fn edge_grips_sit_on_the_center_boundary() {
    let viewport = viewport();
    let mut slots = FiveSlots::default();
    slots.left = open_slot::<Alpha>(TabMount::Eager);
    let layout = layout(&slots, viewport);
    let left = layout.slots.left.unwrap();
    let grip = layout
        .slot_grips
        .iter()
        .find(|grip| grip.side == SlotSide::Left)
        .unwrap();

    assert_eq!(grip.axis, GripAxis::Horizontal);
    assert_eq!(
        grip.rect.right(),
        left.rect.right() + (EDGE_GRIP_PX / 2) as i32
    );
    assert_eq!(grip.start_px, left.rect.w);
    assert_eq!(
        hit_test(
            &layout,
            Vec2Px {
                x: left.rect.right(),
                y: (left.rect.h / 2) as i32,
            }
        ),
        Some(PanelKey::SlotResize(SlotSide::Left))
    );
}

#[test]
fn minimized_resize_edge_exposes_a_restore_grip() {
    let viewport = viewport();
    let mut slots = FiveSlots::default();
    slots.left = open_slot::<Alpha>(TabMount::Eager);
    slots.left.state = SlotState::Minimized;
    let layout = layout(&slots, viewport);
    let grip = layout
        .slot_grips
        .iter()
        .find(|grip| grip.side == SlotSide::Left)
        .unwrap();

    assert_eq!(grip.start_px, 0);
    assert_eq!(
        grip.rect,
        RectPx {
            x: 0,
            y: 0,
            w: EDGE_GRIP_PX,
            h: 500
        }
    );
    assert_eq!(
        hit_test(&layout, Vec2Px { x: 2, y: 250 }),
        Some(PanelKey::SlotResize(SlotSide::Left))
    );
}

#[test]
fn layout_places_minimised_floats_in_the_bar() {
    let mut slots = FiveSlots::default();
    slots.float_panels.insert(float_for::<Alpha>()).unwrap();
    let key = slots.float_panels.key_by_tab::<Alpha>().unwrap();
    let mut queue = CommandQueue::default();
    queue.push_slot(PanelKey::Float(key.raw()), MinimizeSlot);
    queue.apply(&mut slots);

    let layout = layout(&slots, viewport());
    assert!(layout.floats.is_empty());
    assert_eq!(layout.minimized_floats, vec![key.raw()]);
    let bar = layout.minimized_bar.unwrap();
    assert_eq!(
        hit_test(
            &layout,
            Vec2Px {
                x: bar.x + 10,
                y: bar.y + 10
            }
        ),
        Some(PanelKey::Float(key.raw()))
    );
}

#[test]
fn hit_test_uses_open_geometry() {
    let mut slots = FiveSlots::default();
    slots.center = open_slot::<Alpha>(TabMount::Eager);
    let layout = layout(&slots, viewport());
    assert_eq!(
        hit_test(&layout, Vec2Px { x: 500, y: 250 }),
        Some(PanelKey::Slot(SlotSide::Center))
    );
}

#[test]
fn arbitrary_slot_command_results_are_supported() {
    struct CountTabs;

    impl SlotCommand for CountTabs {
        type Out = usize;

        fn apply(self, context: &mut SlotContext<'_>) -> usize {
            context.slot_mut().tab_ids().len()
        }
    }

    let mut slots = FiveSlots::default();
    slots.left = open_slot::<Alpha>(TabMount::Eager);
    let mut context = SlotContext::new(&mut slots.left);
    assert_eq!(CountTabs.apply(&mut context), 2);
}

#[test]
fn title_ops_choose_a_command_scope() {
    let minimize = Minimize.command();
    assert!(matches!(minimize, TitleOpCommand::Slot(_)));
    let close = Close.command();
    assert!(matches!(close, TitleOpCommand::Surface(_)));
}

#[test]
fn float_keys_follow_insertion_and_raise() {
    let mut floats = FloatPanels::default();
    floats.insert(float_for::<Alpha>()).unwrap();
    floats.insert(float_for::<Beta>()).unwrap();
    let first = floats.key_by_tab::<Alpha>().unwrap();
    let second = floats.key_by_tab::<Beta>().unwrap();
    let order: Vec<FloatKey> = floats.iter().map(|(key, _)| key).collect();
    assert_eq!(order, vec![first, second]);
    assert!(floats.raise(first));
    let order: Vec<FloatKey> = floats.iter().map(|(key, _)| key).collect();
    assert_eq!(order, vec![second, first]);
}

#[test]
fn float_rejects_a_second_slot_for_the_same_tab() {
    let mut floats = FloatPanels::default();
    floats.insert(float_for::<Alpha>()).unwrap();
    assert_eq!(
        floats.insert(float_for::<Alpha>()),
        Err(FloatError::DuplicateTab(TypeId::of::<Alpha>()))
    );
}

#[test]
fn center_position_is_percent_based() {
    let panel = FloatPanel {
        size: PercentSize::new(30, 30),
        position: FreePanelPosition::Center,
        ..FloatPanel::default()
    };
    let rect = panel.rect(center(), viewport());
    assert_eq!(
        rect,
        RectPx {
            x: 380,
            y: 190,
            w: 240,
            h: 120
        }
    );
}

#[test]
fn drag_drops_the_axis_that_overflows() {
    let mut panel = FloatPanel {
        position: FreePanelPosition::TopLeft,
        collides_with_edge_slots: true,
        slot: open_slot::<Alpha>(TabMount::Eager),
        ..FloatPanel::default()
    };
    let start = panel.rect(center(), viewport());
    panel.drag(center(), viewport(), start, Vec2Px { x: -60, y: 40 });
    let rect = panel.rect(center(), viewport());
    assert_eq!(rect.x, 100);
    assert!(rect.y > 50);
}

#[test]
fn resize_respects_the_minimum_size() {
    let mut panel = FloatPanel {
        position: FreePanelPosition::TopLeft,
        slot: open_slot::<Alpha>(TabMount::Eager),
        ..FloatPanel::default()
    };
    let start = panel.rect(center(), viewport());
    panel.resize(
        center(),
        viewport(),
        start,
        ResizeEdge::BottomRight,
        Vec2Px { x: -400, y: -400 },
    );
    assert_eq!(panel.size, PercentSize::new(12, 8));
}

#[test]
fn drag_tracks_the_pointer_from_the_gesture_origin() {
    let mut panel = FloatPanel {
        position: FreePanelPosition::Offset(Percent::new(35), Percent::new(4)),
        slot: open_slot::<Alpha>(TabMount::Eager),
        ..FloatPanel::default()
    };
    let viewport = viewport();
    let bounds = float_anchor(viewport);
    let start = panel.rect(bounds, viewport);
    panel.drag(bounds, viewport, start, Vec2Px { x: 40, y: 10 });
    let first = panel.rect(bounds, viewport);
    assert_eq!((first.x, first.y), (start.x + 40, start.y + 10));
    panel.drag(bounds, viewport, start, Vec2Px { x: 80, y: 20 });
    let second = panel.rect(bounds, viewport);
    assert_eq!((second.x, second.y), (start.x + 80, start.y + 20));
}

#[test]
fn drag_keeps_the_float_inside_the_surface() {
    let viewport = viewport();
    let bounds = float_anchor(viewport);
    let mut panel = FloatPanel {
        position: FreePanelPosition::Offset(Percent::new(20), Percent::new(20)),
        slot: open_slot::<Alpha>(TabMount::Eager),
        ..FloatPanel::default()
    };
    let start = panel.rect(bounds, viewport);

    panel.drag(
        bounds,
        viewport,
        start,
        Vec2Px {
            x: 100_000,
            y: 100_000,
        },
    );
    let rect = panel.rect(bounds, viewport);
    assert_eq!(rect.right(), bounds.right());
    assert_eq!(rect.bottom(), bounds.bottom());

    panel.drag(
        bounds,
        viewport,
        rect,
        Vec2Px {
            x: -100_000,
            y: -100_000,
        },
    );
    let rect = panel.rect(bounds, viewport);
    assert_eq!((rect.x, rect.y), (bounds.x, bounds.y));
}

#[test]
fn edge_fill_defaults_to_left_right() {
    assert_eq!(FiveSlots::default().edge_fill_rule, EdgeFillRule::LeftRight);
}

#[test]
fn surfaces_can_use_different_themes() {
    let dark = FiveSlots::default();
    let light = FiveSlots {
        theme: LIGHT,
        ..FiveSlots::default()
    };
    assert_eq!(dark.theme, DARK);
    assert_eq!(light.theme, LIGHT);
    assert_ne!(dark.theme, light.theme);
}

#[test]
fn solver_places_edge_slots_and_center() {
    let mut slots = FiveSlots::default();
    slots.top = open_slot::<Alpha>(TabMount::Eager);
    slots.left = open_slot::<Beta>(TabMount::Eager);
    let rects = slot_rects(&slots, viewport());
    assert_eq!(
        rects.top,
        Some(RectPx {
            x: 220,
            y: 0,
            w: 780,
            h: 110
        })
    );
    assert_eq!(
        rects.left,
        Some(RectPx {
            x: 0,
            y: 0,
            w: 220,
            h: 500
        })
    );
    assert_eq!(
        rects.center,
        RectPx {
            x: 220,
            y: 110,
            w: 780,
            h: 390
        }
    );
}

#[test]
fn top_bottom_rule_matches_the_column_layout() {
    let mut slots = FiveSlots {
        edge_fill_rule: EdgeFillRule::TopBottom,
        ..FiveSlots::default()
    };
    slots.top = open_slot::<Alpha>(TabMount::Eager);
    slots.left = open_slot::<Beta>(TabMount::Eager);
    let rects = slot_rects(&slots, viewport());
    assert_eq!(
        rects.top,
        Some(RectPx {
            x: 0,
            y: 0,
            w: 1_000,
            h: 110
        })
    );
    assert_eq!(
        rects.left,
        Some(RectPx {
            x: 0,
            y: 110,
            w: 220,
            h: 390
        })
    );
    assert_eq!(
        rects.center,
        RectPx {
            x: 220,
            y: 110,
            w: 780,
            h: 390
        }
    );
}

#[test]
fn edge_fill_rules_keep_slots_apart() {
    let viewport = viewport();
    for rule in RULES {
        let slots = all_edges_open(rule);
        assert_slots_are_apart(&slots, viewport, rule);
    }
}

fn rect(x: i32, y: i32, w: u32, h: u32) -> RectPx {
    RectPx { x, y, w, h }
}

#[test]
fn edge_fill_rule_span_table() {
    let viewport = viewport();
    let expected = [
        (
            EdgeFillRule::LeftRight,
            [
                rect(220, 0, 560, 110),
                rect(0, 0, 220, 500),
                rect(780, 0, 220, 500),
                rect(220, 390, 560, 110),
                rect(220, 110, 560, 280),
            ],
        ),
        (
            EdgeFillRule::TopBottom,
            [
                rect(0, 0, 1_000, 110),
                rect(0, 110, 220, 280),
                rect(780, 110, 220, 280),
                rect(0, 390, 1_000, 110),
                rect(220, 110, 560, 280),
            ],
        ),
        (
            EdgeFillRule::TopRight,
            [
                rect(0, 0, 1_000, 110),
                rect(0, 110, 220, 280),
                rect(780, 110, 220, 390),
                rect(220, 390, 560, 110),
                rect(220, 110, 560, 280),
            ],
        ),
        (
            EdgeFillRule::TopLeft,
            [
                rect(0, 0, 1_000, 110),
                rect(0, 110, 220, 390),
                rect(780, 110, 220, 280),
                rect(220, 390, 560, 110),
                rect(220, 110, 560, 280),
            ],
        ),
        (
            EdgeFillRule::BottomRight,
            [
                rect(220, 0, 560, 110),
                rect(0, 110, 220, 280),
                rect(780, 0, 220, 390),
                rect(0, 390, 1_000, 110),
                rect(220, 110, 560, 280),
            ],
        ),
        (
            EdgeFillRule::BottomLeft,
            [
                rect(220, 0, 560, 110),
                rect(0, 0, 220, 390),
                rect(780, 110, 220, 280),
                rect(0, 390, 1_000, 110),
                rect(220, 110, 560, 280),
            ],
        ),
    ];

    for (rule, expected) in expected {
        let slots = all_edges_open(rule);
        assert_eq!(open_rects(&slots, viewport), expected, "rule {rule:?}");
    }
}

#[test]
fn edge_fill_respects_only_occupied_neighbours() {
    let viewport = viewport();
    let mut slots = FiveSlots::default();
    slots.left = open_slot::<Alpha>(TabMount::Eager);
    slots.bottom = open_slot::<Beta>(TabMount::Eager);
    let rects = slot_rects(&slots, viewport);
    let left = rects.left.unwrap();
    let bottom = rects.bottom.unwrap();
    assert_eq!(left.y, 0);
    assert_eq!(left.bottom(), viewport.height_px as i32);
    assert_eq!(bottom.x, left.right());
    assert_eq!(bottom.right(), viewport.width_px as i32);
    assert!(!overlaps(left, bottom));
}

#[test]
fn tiny_viewport_keeps_every_edge_inside_the_surface() {
    let viewport = Viewport {
        width_px: 40,
        height_px: 30,
    };
    for rule in RULES {
        let slots = all_edges_open(rule);
        assert_slots_are_apart(&slots, viewport, rule);
    }
}

#[test]
fn split_title_removes_the_title_bar_height() {
    let (title, content) = split_title(RectPx {
        x: 10,
        y: 20,
        w: 300,
        h: 200,
    });
    assert_eq!(title.h, TITLE_BAR_PX);
    assert_eq!(content.y, 20 + TITLE_BAR_PX as i32);
    assert_eq!(content.h, 200 - TITLE_BAR_PX);
}

#[test]
fn captured_positions_move_a_float_through_the_surface_edge() {
    let viewport = viewport();
    let bounds = float_anchor(viewport);
    let mut slots = FiveSlots::default();
    slots.float_panels.insert(float_for::<Alpha>()).unwrap();
    let keyboard = layout(&slots, viewport).floats[0].raw;
    let target = PanelKey::Float(keyboard);
    let start = layout(&slots, viewport).floats[0].rect;
    let origin = Vec2Px { x: 500, y: 250 };

    let drag_to = |slots: &mut FiveSlots, pointer: Vec2Px| {
        let mut queue = CommandQueue::default();
        queue.push_surface(
            target,
            MoveFloat {
                bounds,
                viewport,
                start,
                delta: Vec2Px {
                    x: pointer.x - origin.x,
                    y: pointer.y - origin.y,
                },
            },
        );
        queue.apply(slots);
    };

    drag_to(&mut slots, Vec2Px { x: 600, y: 300 });
    let panel = slots
        .float_panels
        .float_by_key(FloatKey::from_raw(keyboard))
        .unwrap();
    assert_eq!(
        panel.position,
        FreePanelPosition::Offset(Percent::new(45), Percent::new(45))
    );

    // The pointer leaves the surface, so the panel stops at the edge.
    drag_to(&mut slots, Vec2Px { x: 1_200, y: 700 });
    let panel = slots
        .float_panels
        .float_by_key(FloatKey::from_raw(keyboard))
        .unwrap();
    assert_eq!(
        panel.position,
        FreePanelPosition::Offset(Percent::new(70), Percent::new(70))
    );

    // Returning to the surface resumes from the press origin without a jump.
    drag_to(&mut slots, Vec2Px { x: 600, y: 300 });
    let panel = slots
        .float_panels
        .float_by_key(FloatKey::from_raw(keyboard))
        .unwrap();
    assert_eq!(
        panel.position,
        FreePanelPosition::Offset(Percent::new(45), Percent::new(45))
    );
}
