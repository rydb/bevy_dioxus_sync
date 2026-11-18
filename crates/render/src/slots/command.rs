use std::any::TypeId;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use super::float::{FloatKey, FloatPanel, FloatPanels};
use super::slot::{FiveSlots, Slot, SlotSide, SlotState};
use super::solver::{RectPx, ResizeEdge, Vec2Px, Viewport};
use super::surface::PanelKey;

/// Command applied to the slot that issued it.
pub trait SlotCommand: Send + 'static {
    /// Result returned by the command.
    type Out;

    /// Applies the command to the issuing slot.
    fn apply(self, context: &mut SlotContext<'_>) -> Self::Out;
}

/// Command applied to the surface that hosts the issuing slot.
pub trait SurfaceCommand: Send + 'static {
    /// Result returned by the command.
    type Out;

    /// Applies the command to the issuing slot through a surface context.
    fn apply(self, context: &mut SurfaceContext<'_>) -> Self::Out;
}

/// Type-erased slot command used by the command queue.
#[doc(hidden)]
pub trait ErasedSlotCommand: Send + 'static {
    /// Applies the command to a target.
    fn apply_erased(self: Box<Self>, target: PanelKey, surface: &mut FiveSlots);
}

impl<C: SlotCommand<Out = ()>> ErasedSlotCommand for C {
    fn apply_erased(self: Box<Self>, target: PanelKey, surface: &mut FiveSlots) {
        let command = *self;
        let Some(mut context) = slot_context(surface, target) else {
            return;
        };
        let _ = command.apply(&mut context);
    }
}

/// Type-erased surface command used by the command queue.
#[doc(hidden)]
pub trait ErasedSurfaceCommand: Send + 'static {
    /// Applies the command to a target.
    fn apply_erased(self: Box<Self>, target: PanelKey, surface: &mut FiveSlots);
}

impl<C: SurfaceCommand<Out = ()>> ErasedSurfaceCommand for C {
    fn apply_erased(self: Box<Self>, target: PanelKey, surface: &mut FiveSlots) {
        let command = *self;
        let mut context = SurfaceContext::new(surface, target);
        let _ = command.apply(&mut context);
    }
}

/// Command produced by a title bar action.
pub enum TitleOpCommand {
    /// A command limited to the issuing slot.
    Slot(Box<dyn ErasedSlotCommand>),
    /// A command that may use the surface.
    Surface(Box<dyn ErasedSurfaceCommand>),
}

impl TitleOpCommand {
    /// Builds a slot command.
    pub fn slot(command: impl SlotCommand<Out = ()> + 'static) -> Self {
        Self::Slot(Box::new(command))
    }

    /// Builds a surface command.
    pub fn surface(command: impl SurfaceCommand<Out = ()> + 'static) -> Self {
        Self::Surface(Box::new(command))
    }
}

/// Execution context limited to one slot.
pub struct SlotContext<'a> {
    slot: &'a mut Slot,
}

impl<'a> SlotContext<'a> {
    pub(crate) fn new(slot: &'a mut Slot) -> Self {
        Self { slot }
    }

    /// Returns the issuing slot.
    pub fn slot(&self) -> &Slot {
        self.slot
    }

    /// Returns the issuing slot mutably.
    pub fn slot_mut(&mut self) -> &mut Slot {
        self.slot
    }
}

/// Execution context for the issuing slot and its container.
pub struct SurfaceContext<'a> {
    surface: &'a mut FiveSlots,
    target: PanelKey,
}

impl<'a> SurfaceContext<'a> {
    pub(crate) fn new(surface: &'a mut FiveSlots, target: PanelKey) -> Self {
        Self { surface, target }
    }

    /// Returns the identity of the issuing slot.
    pub fn target(&self) -> PanelKey {
        self.target
    }

    /// Returns the issuing slot.
    pub fn slot(&self) -> Option<&Slot> {
        match self.target {
            PanelKey::Slot(side) | PanelKey::SlotTab(side, _) | PanelKey::SlotResize(side) => {
                Some(self.surface.slot(side))
            }
            PanelKey::Float(raw) | PanelKey::FloatTab(raw, _) | PanelKey::FloatResize(raw, _) => {
                self.surface
                    .float_panels
                    .float_by_key(FloatKey::from_raw(raw))
                    .map(|panel| &panel.slot)
            }
        }
    }

    /// Returns the issuing slot mutably.
    pub fn slot_mut(&mut self) -> Option<&mut Slot> {
        match self.target {
            PanelKey::Slot(side) | PanelKey::SlotTab(side, _) | PanelKey::SlotResize(side) => {
                Some(self.surface.slot_mut(side))
            }
            PanelKey::Float(raw) | PanelKey::FloatTab(raw, _) | PanelKey::FloatResize(raw, _) => {
                self.surface
                    .float_panels
                    .get_mut(FloatKey::from_raw(raw))
                    .map(|panel| &mut panel.slot)
            }
        }
    }

    /// Returns the issuing floating slot.
    pub fn float(&self) -> Option<&FloatPanel> {
        match self.target {
            PanelKey::Float(raw) | PanelKey::FloatTab(raw, _) | PanelKey::FloatResize(raw, _) => {
                self.surface
                    .float_panels
                    .float_by_key(FloatKey::from_raw(raw))
            }
            PanelKey::Slot(_) | PanelKey::SlotTab(_, _) | PanelKey::SlotResize(_) => None,
        }
    }

    /// Returns the issuing floating slot mutably.
    pub fn float_mut(&mut self) -> Option<&mut FloatPanel> {
        match self.target {
            PanelKey::Float(raw) | PanelKey::FloatTab(raw, _) | PanelKey::FloatResize(raw, _) => {
                self.surface.float_panels.get_mut(FloatKey::from_raw(raw))
            }
            PanelKey::Slot(_) | PanelKey::SlotTab(_, _) | PanelKey::SlotResize(_) => None,
        }
    }

    /// Returns the floating collection.
    pub fn float_panels_mut(&mut self) -> &mut FloatPanels {
        &mut self.surface.float_panels
    }

    /// Resizes one edge slot.
    pub fn resize_edge(
        &mut self,
        side: SlotSide,
        viewport: Viewport,
        start_px: u32,
        delta_px: i32,
    ) {
        self.surface.resize_edge(side, viewport, start_px, delta_px);
    }
}

fn float_key(target: PanelKey) -> Option<FloatKey> {
    match target {
        PanelKey::Float(raw) | PanelKey::FloatTab(raw, _) | PanelKey::FloatResize(raw, _) => {
            Some(FloatKey::from_raw(raw))
        }
        PanelKey::Slot(_) | PanelKey::SlotTab(_, _) | PanelKey::SlotResize(_) => None,
    }
}

fn slot_context<'a>(surface: &'a mut FiveSlots, target: PanelKey) -> Option<SlotContext<'a>> {
    match target {
        PanelKey::Slot(side) | PanelKey::SlotTab(side, _) | PanelKey::SlotResize(side) => {
            Some(SlotContext::new(surface.slot_mut(side)))
        }
        PanelKey::Float(raw) | PanelKey::FloatTab(raw, _) | PanelKey::FloatResize(raw, _) => {
            surface
                .float_panels
                .get_mut(FloatKey::from_raw(raw))
                .map(|panel| SlotContext::new(&mut panel.slot))
        }
    }
}

/// Selects a tab inside the issuing slot.
pub struct SelectTab(pub TypeId);

impl SlotCommand for SelectTab {
    type Out = ();

    fn apply(self, context: &mut SlotContext<'_>) {
        context.slot_mut().select_id(self.0);
    }
}

/// Hides the issuing slot without touching its active tab.
pub struct MinimizeSlot;

impl SlotCommand for MinimizeSlot {
    type Out = ();

    fn apply(self, context: &mut SlotContext<'_>) {
        context.slot_mut().state = SlotState::Minimized;
    }
}

/// Shows the issuing slot without touching its active tab.
pub struct OpenSlot;

impl SlotCommand for OpenSlot {
    type Out = ();

    fn apply(self, context: &mut SlotContext<'_>) {
        context.slot_mut().state = SlotState::Open;
    }
}

/// Closes the issuing slot.
pub struct CloseSlot;

impl SurfaceCommand for CloseSlot {
    type Out = ();

    fn apply(self, context: &mut SurfaceContext<'_>) {
        match float_key(context.target()) {
            Some(key) => {
                context.float_panels_mut().close(key);
            }
            None => {
                if let Some(slot) = context.slot_mut() {
                    slot.state = SlotState::Closed;
                }
            }
        }
    }
}

/// Raises the issuing floating slot.
pub struct RaiseFloat;

impl SurfaceCommand for RaiseFloat {
    type Out = ();

    fn apply(self, context: &mut SurfaceContext<'_>) {
        if let Some(key) = float_key(context.target()) {
            context.float_panels_mut().raise(key);
        }
    }
}

/// Moves the issuing floating slot.
pub struct MoveFloat {
    pub bounds: RectPx,
    pub viewport: Viewport,
    pub start: RectPx,
    pub delta: Vec2Px,
}

impl SurfaceCommand for MoveFloat {
    type Out = ();

    fn apply(self, context: &mut SurfaceContext<'_>) {
        if let Some(key) = float_key(context.target()) {
            context.float_panels_mut().drag(
                key,
                self.bounds,
                self.viewport,
                self.start,
                self.delta,
            );
        }
    }
}

/// Resizes the issuing floating slot.
pub struct ResizeFloat {
    pub bounds: RectPx,
    pub viewport: Viewport,
    pub start: RectPx,
    pub edge: ResizeEdge,
    pub delta: Vec2Px,
}

impl SurfaceCommand for ResizeFloat {
    type Out = ();

    fn apply(self, context: &mut SurfaceContext<'_>) {
        if let Some(key) = float_key(context.target()) {
            context.float_panels_mut().resize(
                key,
                self.bounds,
                self.viewport,
                self.start,
                self.edge,
                self.delta,
            );
        }
    }
}

/// Resizes one edge slot from a pixel start.
pub struct ResizeSlot {
    pub side: SlotSide,
    pub viewport: Viewport,
    pub start_px: u32,
    pub delta_px: i32,
}

impl SurfaceCommand for ResizeSlot {
    type Out = ();

    fn apply(self, context: &mut SurfaceContext<'_>) {
        context.resize_edge(self.side, self.viewport, self.start_px, self.delta_px);
    }
}

pub(crate) struct SlotJob {
    target: PanelKey,
    command: Box<dyn ErasedSlotCommand>,
}

pub(crate) struct SurfaceJob {
    target: PanelKey,
    command: Box<dyn ErasedSurfaceCommand>,
}

/// Batched commands waiting for the surface root to apply them.
#[derive(Default)]
pub struct CommandQueue {
    slots: Vec<SlotJob>,
    surfaces: Vec<SurfaceJob>,
}

impl CommandQueue {
    /// Queues a slot command for a target.
    pub fn push_slot(&mut self, target: PanelKey, command: impl SlotCommand<Out = ()> + 'static) {
        self.slots.push(SlotJob {
            target,
            command: Box::new(command),
        });
    }

    /// Queues a surface command for a target.
    pub fn push_surface(
        &mut self,
        target: PanelKey,
        command: impl SurfaceCommand<Out = ()> + 'static,
    ) {
        self.surfaces.push(SurfaceJob {
            target,
            command: Box::new(command),
        });
    }

    /// Queues an already boxed slot command.
    pub fn push_slot_boxed(&mut self, target: PanelKey, command: Box<dyn ErasedSlotCommand>) {
        self.slots.push(SlotJob { target, command });
    }

    /// Queues an already boxed surface command.
    pub fn push_surface_boxed(&mut self, target: PanelKey, command: Box<dyn ErasedSurfaceCommand>) {
        self.surfaces.push(SurfaceJob { target, command });
    }

    pub(crate) fn push_slot_job(&mut self, job: SlotJob) {
        self.slots.push(job);
    }

    pub(crate) fn push_surface_job(&mut self, job: SurfaceJob) {
        self.surfaces.push(job);
    }

    /// Returns whether the queue has no commands.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty() && self.surfaces.is_empty()
    }

    /// Applies every queued command in insertion order.
    pub fn apply(&mut self, surface: &mut FiveSlots) {
        for job in self.slots.drain(..) {
            job.command.apply_erased(job.target, surface);
        }
        for job in self.surfaces.drain(..) {
            job.command.apply_erased(job.target, surface);
        }
    }
}

/// Sends commands to the surface root.
#[derive(Clone)]
pub struct CommandSender {
    slot_tx: UnboundedSender<SlotJob>,
    surface_tx: UnboundedSender<SurfaceJob>,
}

impl CommandSender {
    /// Sends a slot command for a target.
    pub fn send_slot(&self, target: PanelKey, command: impl SlotCommand<Out = ()> + 'static) {
        let _ = self.slot_tx.send(SlotJob {
            target,
            command: Box::new(command),
        });
    }

    /// Sends an already boxed slot command.
    pub fn send_slot_boxed(&self, target: PanelKey, command: Box<dyn ErasedSlotCommand>) {
        let _ = self.slot_tx.send(SlotJob { target, command });
    }

    /// Sends a surface command for a target.
    pub fn send_surface(&self, target: PanelKey, command: impl SurfaceCommand<Out = ()> + 'static) {
        let _ = self.surface_tx.send(SurfaceJob {
            target,
            command: Box::new(command),
        });
    }

    /// Sends an already boxed surface command.
    pub fn send_surface_boxed(&self, target: PanelKey, command: Box<dyn ErasedSurfaceCommand>) {
        let _ = self.surface_tx.send(SurfaceJob { target, command });
    }
}

/// Receives commands at the surface root.
pub struct CommandReceiver {
    pub(crate) slot_rx: UnboundedReceiver<SlotJob>,
    pub(crate) surface_rx: UnboundedReceiver<SurfaceJob>,
}

impl CommandReceiver {
    /// Creates a connected sender and receiver.
    pub fn channel() -> (CommandSender, CommandReceiver) {
        let (slot_tx, slot_rx) = unbounded_channel();
        let (surface_tx, surface_rx) = unbounded_channel();
        (
            CommandSender {
                slot_tx,
                surface_tx,
            },
            CommandReceiver {
                slot_rx,
                surface_rx,
            },
        )
    }
}
