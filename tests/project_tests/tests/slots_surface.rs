//! Public surface checks for the five slot panel state.

use std::any::TypeId;

use bevy_dioxus_render::slots::{
    ActiveTab, CloseSlot, CommandQueue, FiveSlots, FloatPanel, FreePanelPosition, MinimizeSlot,
    Name, OpenSlot, PanelKey, PercentSize, SelectTab, Slot, SlotSide, SlotState, Tab, TabFn,
    TabGroup, TabMount, TitleBar, TitleOps, Viewport, hit_test, layout, tab_entry,
};
use dioxus_core::{Element, VNode};

fn body() -> Element {
    Ok(VNode::placeholder())
}

#[derive(Name)]
struct Inspector;

#[derive(Name)]
struct Scratch;

impl Tab for Inspector {
    fn element() -> TabFn {
        body
    }
}

impl Tab for Scratch {
    fn element() -> TabFn {
        body
    }
}

fn enabled<T: Tab>() -> TitleBar {
    TitleBar::Enabled {
        group: TabGroup::from_entries([tab_entry::<T>()], TabMount::Eager),
        ops: TitleOps::empty(),
    }
}

fn slot<T: Tab>() -> Slot {
    Slot {
        title_bar: enabled::<T>(),
        active_tab: Some(ActiveTab::new::<T>()),
        state: SlotState::Open,
    }
}

fn viewport() -> Viewport {
    Viewport {
        width_px: 1_000,
        height_px: 500,
    }
}

#[test]
fn tab_entry_infers_the_tab_title_from_the_type_name() {
    let (_, entry) = tab_entry::<Inspector>();
    assert_eq!(entry.title, "Inspector");
}

#[test]
fn docked_state_commands_keep_the_active_tab() {
    let mut slots = FiveSlots::default();
    slots.left = slot::<Inspector>();
    let target = PanelKey::Slot(SlotSide::Left);

    let mut queue = CommandQueue::default();
    queue.push_slot(target, MinimizeSlot);
    queue.apply(&mut slots);
    assert_eq!(slots.left.state, SlotState::Minimized);
    assert_eq!(slots.left.active_id(), Some(TypeId::of::<Inspector>()));

    let mut queue = CommandQueue::default();
    queue.push_slot(target, OpenSlot);
    queue.apply(&mut slots);
    assert_eq!(slots.left.state, SlotState::Open);

    let mut queue = CommandQueue::default();
    queue.push_surface(target, CloseSlot);
    queue.apply(&mut slots);
    assert_eq!(slots.left.state, SlotState::Closed);
    assert_eq!(slots.left.active_id(), Some(TypeId::of::<Inspector>()));
}

#[test]
fn selecting_a_tab_does_not_change_the_slot_state() {
    let mut slots = FiveSlots::default();
    slots.left = slot::<Scratch>();
    slots.left.state = SlotState::Minimized;
    let mut queue = CommandQueue::default();
    queue.push_slot(
        PanelKey::Slot(SlotSide::Left),
        SelectTab(TypeId::of::<Scratch>()),
    );
    queue.apply(&mut slots);
    assert_eq!(slots.left.state, SlotState::Minimized);
}

#[test]
fn float_commands_use_geometry_keys() {
    let mut slots = FiveSlots::default();
    slots
        .float_panels
        .insert(FloatPanel {
            position: FreePanelPosition::Center,
            size: PercentSize::new(30, 30),
            slot: slot::<Scratch>(),
            ..FloatPanel::default()
        })
        .unwrap();

    let raw = layout(&slots, viewport()).floats[0].raw;
    let target = PanelKey::Float(raw);

    let mut queue = CommandQueue::default();
    queue.push_slot(target, MinimizeSlot);
    queue.apply(&mut slots);
    assert!(layout(&slots, viewport()).minimized_floats.contains(&raw));

    let mut queue = CommandQueue::default();
    queue.push_slot(target, OpenSlot);
    queue.apply(&mut slots);
    assert_eq!(
        slots
            .float_panels
            .float_by_tab::<Scratch>()
            .unwrap()
            .slot
            .state,
        SlotState::Open
    );

    let mut queue = CommandQueue::default();
    queue.push_surface(target, CloseSlot);
    queue.apply(&mut slots);
    assert!(slots.float_panels.is_empty());
}

#[test]
fn hit_test_reports_the_open_slot() {
    let mut slots = FiveSlots::default();
    slots.center = slot::<Inspector>();
    let surface = layout(&slots, viewport());
    assert_eq!(
        hit_test(
            &surface,
            bevy_dioxus_render::slots::Vec2Px { x: 500, y: 250 }
        ),
        Some(PanelKey::Slot(SlotSide::Center))
    );
}
