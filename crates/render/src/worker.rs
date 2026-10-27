use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyrender_vello::VelloScenePainter;
use bevy_dioxus_interop::DioxusMessage;
use bevy_dioxus_tracing::{debug, error, warn};
use bevy_ecs::prelude::*;
use blitz_dom::Document;
use blitz_paint::paint_scene;
use blitz_traits::events::{DomEvent, DomEventData, UiEvent};
use blitz_traits::node_id::NodeId;
use blitz_traits::shell::Viewport;
use crossbeam_channel::{Receiver, Sender};
use dioxus_devtools::DevserverMsg;
use dioxus_native::DioxusDocument;
use vello::Scene;

use keyboard_types::{Key, NamedKey};

use crate::draft_value::{self, Draft};
use crate::select_all_on_focus::select_all_on_focus;
use crate::text_input::select_all_text;
use crate::{COLOR_SCHEME, SCALE_FACTOR, document_owns_point};

/// Extracts page coordinates from a UI event for hit-testing.
fn extract_ui_event_coords(event: &UiEvent) -> (f32, f32) {
    match event {
        UiEvent::PointerMove(e) | UiEvent::PointerDown(e) | UiEvent::PointerUp(e) => {
            (e.coords.page_x, e.coords.page_y)
        }
        _ => (0.0, 0.0),
    }
}

/// Turns an event into one that edits a draft input without telling the app.
///
/// The event is returned unchanged when it does not edit a draft input.
fn quiet_event(dioxus_doc: &DioxusDocument, event: UiEvent) -> Result<(NodeId, DomEvent), UiEvent> {
    let Some(node_id) = draft_value::quiet_target(dioxus_doc) else {
        return Err(event);
    };

    let data = match event {
        UiEvent::KeyDown(event) if draft_value::edits_text(&event) => DomEventData::KeyDown(event),
        UiEvent::Ime(event) => DomEventData::Ime(event),
        UiEvent::AppleStandardKeybinding(command) => DomEventData::AppleStandardKeybinding(command),
        other => return Err(other),
    };

    Ok((node_id, DomEvent::new(node_id, data)))
}

/// Transfers ownership of a `!Send` value into a spawned thread.
///
/// Wrap a value with [`SendToThread::new`], move the wrapper into a
/// [`std::thread::spawn`] closure, then call [`SendToThread::take`] to
/// extract the value on the destination thread. If the value is never
/// claimed, for example when thread spawn fails, it is dropped with the
/// wrapper on the current thread.
struct SendToThread<T> {
    value: Option<T>,
}

// SAFETY: SendToThread<T> can be sent between threads because the inner
// value is only accessed through take(), which moves it out exactly once.
unsafe impl<T> Send for SendToThread<T> {}

impl<T> SendToThread<T> {
    /// Wrap a value for transfer to another thread.
    fn new(value: T) -> Self {
        SendToThread { value: Some(value) }
    }

    /// Claim ownership on the current thread and extract the inner value.
    ///
    /// # Panics
    ///
    /// Panics if called more than once.
    fn take(mut self) -> T {
        self.value
            .take()
            .expect("SendToThread::take() called more than once")
    }
}

/// Commands sent from the main thread to a VDOM worker.
#[derive(Debug)]
pub enum VdomCommand {
    /// Poll the VDOM, resolve animations, paint, and send the scene back.
    Poll {
        /// Frame timestamp used for animation resolution.
        animation_time: f64,
    },
    /// Mirrored values that include the worker's last edit are published.
    Echoed,
    /// Forward a dioxus message received on the main thread.
    Message(DioxusMessage),
    /// Update the viewport dimensions.
    Resize(u32, u32),
    /// Stop the worker thread and drop the VDOM.
    Shutdown,
}

/// Results sent from the worker back to the main thread.
pub enum VdomResult {
    /// A painted scene ready for GPU rendering.
    SceneReady {
        scene: Scene,
        width: u32,
        height: u32,
    },
    /// Hit-test result: whether a DOM element with catch-events was under the pointer.
    HitTestResult { entity: Entity, caught: bool },
    /// The worker edited its document and needs a sync tick before rendering.
    NeedsEcho { entity: Entity },
    /// Worker confirms shutdown.
    ShutdownAck,
}

/// Handle to a running VDOM worker thread.
#[derive(Debug)]
pub struct VdomWorker {
    /// Channel for sending commands into the worker.
    pub cmd_tx: Sender<VdomCommand>,
    /// Channel for receiving results from the worker.
    pub result_rx: Receiver<VdomResult>,
    /// Channel for forwarding input events to the worker.
    pub input_tx: Sender<(Entity, UiEvent)>,
    /// Flag set by the worker's waker when dioxus futures resolve.
    pub waker_flag: Arc<AtomicBool>,
    /// Join handle for the worker thread.
    pub thread: Option<std::thread::JoinHandle<()>>,
}

/// Registry of all active VDOM worker threads.
#[derive(Resource, Default, Debug)]
pub struct VdomThreadRegistry {
    pub workers: HashMap<Entity, VdomWorker>,
}

impl VdomWorker {
    /// Spawn a new OS thread that owns the VDOM and runs the poll and paint
    /// loop. The `messages_recv` channel receives messages from the document
    /// proxy and from devtools, processed directly inside the worker.
    pub fn spawn(
        entity: Entity,
        document: DioxusDocument,
        messages_recv: Receiver<DioxusMessage>,
        cmd_rx: Receiver<VdomCommand>,
        result_tx: Sender<VdomResult>,
        input_rx: Receiver<(Entity, UiEvent)>,
        waker_flag: Arc<AtomicBool>,
    ) -> std::thread::JoinHandle<()> {
        let sendable = SendToThread::new(document);

        std::thread::Builder::new()
            .name(format!("vdom-worker-{}", entity.index()))
            .spawn(move || {
                let mut document = sendable.take();

                struct WorkerWaker {
                    flag: Arc<AtomicBool>,
                }
                impl std::task::Wake for WorkerWaker {
                    fn wake(self: Arc<Self>) {
                        self.flag.store(true, Ordering::SeqCst);
                    }
                }
                let waker = std::task::Waker::from(Arc::new(WorkerWaker {
                    flag: waker_flag.clone(),
                }));

                let mut scene = Scene::new();
                // Text kept across renders for an input being edited.
                let mut draft: Option<Draft> = None;
                // Field whose value stays selected until the next interaction.
                let mut pending_selection: Option<NodeId> = None;
                // Repaint request carried across command iterations.
                let mut needs_paint = false;
                // Timestamp of the most recent poll, reused by the next paint.
                let mut animation_time = 0.0;
                // Edits wait for their mirrored values before the VDOM renders.
                let mut await_echo = false;

                loop {
                    let cmd = match cmd_rx.recv() {
                        Ok(cmd) => cmd,
                        Err(_) => {
                            debug!("vdom-worker-{}: cmd channel closed", entity.index());
                            return;
                        }
                    };

                    let mut should_paint = false;

                    match cmd {
                        VdomCommand::Shutdown => {
                            let _ = result_tx.send(VdomResult::ShutdownAck);
                            return;
                        }
                        VdomCommand::Resize(w, h) => {
                            document.inner.borrow_mut().set_viewport(Viewport::new(
                                w,
                                h,
                                SCALE_FACTOR,
                                COLOR_SCHEME,
                            ));
                            needs_paint = true;
                        }
                        VdomCommand::Message(msg) => {
                            process_dioxus_message(&mut document, msg, &waker);
                            needs_paint = true;
                        }
                        VdomCommand::Poll { animation_time: t } => {
                            animation_time = t;

                            if await_echo {
                                needs_paint = true;
                            } else {
                                if run_poll(&mut document, &waker, &waker_flag) {
                                    needs_paint = true;
                                }

                                // A render can write the mirrored value over the
                                // text being edited, so the draft is put back.
                                if draft_value::restore(&mut document, &draft) {
                                    needs_paint = true;
                                }

                                // A render can also drop the initial selection,
                                // which is put back until the next interaction.
                                if let Some(node) = pending_selection {
                                    if document.inner.borrow().get_focussed_node_id() == Some(node)
                                    {
                                        select_all_text(&mut document, node);
                                    } else {
                                        pending_selection = None;
                                    }
                                }
                            }

                            should_paint = true;
                        }
                        VdomCommand::Echoed => {
                            await_echo = false;
                        }
                    }

                    // Input is handled after polling so the VDOM cannot write a
                    // mirrored value back over an edit the main thread has not
                    // applied yet.
                    let mut edited = false;
                    let mut submitted = false;

                    while let Ok((ev_entity, ui_event)) = input_rx.try_recv() {
                        let (x, y) = extract_ui_event_coords(&ui_event);
                        let caught = document
                            .inner
                            .borrow()
                            .hit(x, y)
                            .map(|hit| document_owns_point(&document, hit.node_id))
                            .unwrap_or(false);
                        let opens_input = matches!(&ui_event, UiEvent::PointerDown(_));
                        let submits = matches!(
                            &ui_event,
                            UiEvent::KeyDown(event) if event.key == Key::Named(NamedKey::Enter)
                        );

                        // Anything that can move the caret ends the initial
                        // selection, which is only meant to last until then.
                        if matches!(
                            &ui_event,
                            UiEvent::PointerDown(_)
                                | UiEvent::KeyDown(_)
                                | UiEvent::Ime(_)
                                | UiEvent::AppleStandardKeybinding(_)
                        ) {
                            pending_selection = None;
                        }

                        let focus_before = document.inner.borrow().get_focussed_node_id();

                        // A draft input applies its edits without telling the app,
                        // which hears the kept value once the field is committed.
                        match quiet_event(&document, ui_event) {
                            Ok((_, mut event)) => {
                                document
                                    .inner
                                    .borrow_mut()
                                    .handle_dom_event(&mut event, |_| {});
                            }
                            Err(ui_event) => {
                                document.handle_ui_event(ui_event);
                                edited = true;
                            }
                        }

                        // A pointer press grants focus in its default action, so
                        // the value is selected once the event has been handled.
                        if opens_input
                            && document.inner.borrow().get_focussed_node_id() != focus_before
                        {
                            pending_selection = select_all_on_focus(&mut document);
                        }

                        needs_paint = true;
                        should_paint = true;
                        submitted |= submits;
                        let _ = result_tx.try_send(VdomResult::HitTestResult {
                            entity: ev_entity,
                            caught,
                        });
                    }

                    draft_value::track(&mut document, &mut draft);

                    if submitted {
                        draft_value::finish(&mut document, &mut draft);
                    }

                    if edited {
                        await_echo = true;
                        let _ = result_tx.try_send(VdomResult::NeedsEcho { entity });
                    }

                    while let Ok(msg) = messages_recv.try_recv() {
                        process_dioxus_message(&mut document, msg, &waker);
                        needs_paint = true;
                        should_paint = true;
                    }

                    if should_paint && needs_paint {
                        scene = paint(&mut document, animation_time, scene, &result_tx);
                        needs_paint = false;
                    }
                }
            })
            .expect("failed to spawn vdom worker thread")
    }
}

/// Process a single dioxus message inside the worker.
fn process_dioxus_message(doc: &mut DioxusDocument, msg: DioxusMessage, waker: &std::task::Waker) {
    match msg {
        DioxusMessage::Devserver(devserver_msg) => match devserver_msg {
            DevserverMsg::HotReload(hotreload_message) => {
                dioxus_devtools::apply_changes(&doc.vdom, &hotreload_message);
                for asset_path in &hotreload_message.assets {
                    if let Some(url) = asset_path.to_str() {
                        doc.inner.borrow_mut().reload_resource_by_href(url);
                    }
                }
            }
            DevserverMsg::FullReloadStart => {}
            _ => {}
        },
        DioxusMessage::CreateHeadElement(el) => {
            doc.create_head_element(&el.name, &el.attributes, &el.contents);
            doc.poll(Some(std::task::Context::from_waker(waker)));
        }
        DioxusMessage::ResourceLoad(resource) => {
            doc.inner
                .borrow_mut()
                .load_resource(blitz_dom::net::ResourceLoadResponse {
                    request_id: 0,
                    node_id: None,
                    resolved_url: None,
                    result: Ok(resource.clone()),
                });
        }
    }
}

/// Polls the VDOM until no more futures are ready.
fn run_poll(doc: &mut DioxusDocument, waker: &std::task::Waker, waker_flag: &AtomicBool) -> bool {
    let mut had_work = false;

    loop {
        waker_flag.store(false, Ordering::SeqCst);
        if doc.poll(Some(std::task::Context::from_waker(waker))) {
            had_work = true;
        }
        if !waker_flag.load(Ordering::SeqCst) {
            break;
        }
    }

    had_work
}

/// Resolves animations and paints the document into `scene`.
fn paint(
    doc: &mut DioxusDocument,
    animation_time: f64,
    mut scene: Scene,
    result_tx: &Sender<VdomResult>,
) -> Scene {
    doc.inner.borrow_mut().resolve(animation_time);

    let (width, height) = {
        let inner = doc.inner.borrow();
        let vp = inner.viewport();
        (vp.window_size.0, vp.window_size.1)
    };

    if width == 0 || height == 0 {
        warn!("vdom worker: zero-size viewport, skipping paint");
        return scene;
    }

    scene.reset();
    paint_scene(
        &mut VelloScenePainter::new(&mut scene),
        &mut *doc.inner.borrow_mut(),
        SCALE_FACTOR as f64,
        width,
        height,
        0,
        0,
    );

    if result_tx
        .send(VdomResult::SceneReady {
            scene,
            width,
            height,
        })
        .is_err()
    {
        error!("vdom worker: result channel closed");
    }

    Scene::new()
}
