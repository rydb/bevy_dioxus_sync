use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::ThreadId;

use anyrender_vello::VelloScenePainter;
use bevy_dioxus_interop::DioxusMessage;
use bevy_dioxus_tracing::{debug, error, warn};
use bevy_ecs::prelude::*;
use blitz_dom::Document;
use blitz_paint::paint_scene;
use blitz_traits::events::UiEvent;
use blitz_traits::shell::Viewport;
use crossbeam_channel::{Receiver, Sender};
use dioxus_devtools::DevserverMsg;
use dioxus_native::DioxusDocument;
use vello::Scene;

use crate::{COLOR_SCHEME, SCALE_FACTOR};

/// Placeholder animation time for intra-frame re-polls from future wakeups.
const ANIMATION_TIME_PLACEHOLDER: f64 = 0.0;

/// Transfers ownership of a `!Send` value into a spawned thread.
///
/// Wrap a value with [`SendToThread::new`], move the wrapper into a
/// [`std::thread::spawn`] closure, then call [`SendToThread::take`] to
/// extract the value on the destination thread.
///
/// # Panics
///
/// Panics if [`take`](Self::take) is called more than once, or if the
/// wrapper is dropped without [`take`](Self::take) being called.
struct SendToThread<T> {
    value: ManuallyDrop<T>,
    owner: Option<ThreadId>,
}

// SAFETY: SendToThread<T> can be sent between threads because the inner
// value is never accessed until take() is called, and take() panics if
// called from a thread other than the first one that calls it.
unsafe impl<T> Send for SendToThread<T> {}

impl<T> SendToThread<T> {
    /// Wrap a value for transfer to another thread.
    fn new(value: T) -> Self {
        SendToThread {
            value: ManuallyDrop::new(value),
            owner: None,
        }
    }

    /// Claim ownership on the current thread and extract the inner value.
    ///
    /// # Panics
    ///
    /// Panics if called more than once.
    fn take(mut self) -> T {
        assert!(
            self.owner.is_none(),
            "SendToThread::take() called more than once"
        );
        self.owner = Some(std::thread::current().id());
        let value = unsafe { ManuallyDrop::take(&mut self.value) };
        std::mem::forget(self);
        value
    }
}

impl<T> Drop for SendToThread<T> {
    fn drop(&mut self) {
        match self.owner {
            Some(owner) => {
                let current = std::thread::current().id();
                assert_eq!(
                    current, owner,
                    "SendToThread dropped on thread {current:?}, expected {owner:?}"
                );
                unsafe { ManuallyDrop::drop(&mut self.value) };
            }
            None => {
                panic!("SendToThread dropped without take() being called");
            }
        }
    }
}

/// Commands sent from the main thread to a VDOM worker.
pub enum VdomCommand {
    /// Poll the VDOM, resolve animations, paint, and send the scene back.
    Poll {
        /// Frame timestamp used for animation resolution.
        animation_time: f64,
    },
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
    /// The worker caught an input event this frame.
    /// The entity identifies which document caught it.
    InputCaught,
    /// Worker confirms shutdown.
    ShutdownAck,
}

/// Handle to a running VDOM worker thread.
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
///
/// Replaces the previous single-threaded document map as the ECS resource
/// that tracks VDOM state per entity.
#[derive(Resource, Default)]
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
                let waker =
                    std::task::Waker::from(Arc::new(WorkerWaker {
                        flag: waker_flag.clone(),
                    }));

                let mut scene = Scene::new();

                loop {
                    // Process input events before polling.
                    while let Ok((_ev_entity, ui_event)) =
                        input_rx.try_recv()
                    {
                        document.handle_ui_event(ui_event);
                        // Report that input was caught by this document.
                        let _ = result_tx
                            .try_send(VdomResult::InputCaught);
                    }

                    while let Ok(msg) = messages_recv.try_recv() {
                        process_dioxus_message(
                            &mut document,
                            msg,
                            &waker,
                        );
                    }

                    while let Ok(cmd) = cmd_rx.try_recv() {
                        match cmd {
                            VdomCommand::Shutdown => {
                                let _ = result_tx
                                    .send(VdomResult::ShutdownAck);
                                return;
                            }
                            VdomCommand::Resize(w, h) => {
                                document
                                    .inner
                                    .borrow_mut()
                                    .set_viewport(Viewport::new(
                                        w, h,
                                        SCALE_FACTOR,
                                        COLOR_SCHEME,
                                    ));
                            }
                            VdomCommand::Message(msg) => {
                                process_dioxus_message(
                                    &mut document,
                                    msg,
                                    &waker,
                                );
                            }
                            VdomCommand::Poll { animation_time } => {
                                let fresh = run_poll_and_paint(
                                    &mut document,
                                    &waker,
                                    &waker_flag,
                                    animation_time,
                                    scene,
                                    &result_tx,
                                );
                                scene = fresh;
                            }
                        }
                    }

                    if waker_flag.load(Ordering::SeqCst) {
                        let fresh = run_poll_and_paint(
                            &mut document,
                            &waker,
                            &waker_flag,
                            ANIMATION_TIME_PLACEHOLDER,
                            scene,
                            &result_tx,
                        );
                        scene = fresh;
                        continue;
                    }

                    match cmd_rx.recv() {
                        Ok(VdomCommand::Shutdown) => {
                            let _ =
                                result_tx.send(VdomResult::ShutdownAck);
                            return;
                        }
                        Ok(VdomCommand::Poll { animation_time }) => {
                            let fresh = run_poll_and_paint(
                                &mut document,
                                &waker,
                                &waker_flag,
                                animation_time,
                                scene,
                                &result_tx,
                            );
                            scene = fresh;
                        }
                        Ok(VdomCommand::Resize(w, h)) => {
                            document
                                .inner
                                .borrow_mut()
                                .set_viewport(Viewport::new(
                                    w, h,
                                    SCALE_FACTOR,
                                    COLOR_SCHEME,
                                ));
                        }
                        Ok(VdomCommand::Message(msg)) => {
                            process_dioxus_message(
                                &mut document,
                                msg,
                                &waker,
                            );
                        }
                        Err(_) => {
                            debug!(
                                "vdom-worker-{}: cmd channel closed",
                                entity.index()
                            );
                            return;
                        }
                    }
                }
            })
            .expect("failed to spawn vdom worker thread")
    }
}

/// Process a single dioxus message inside the worker.
fn process_dioxus_message(
    doc: &mut DioxusDocument,
    msg: DioxusMessage,
    waker: &std::task::Waker,
) {
    match msg {
        DioxusMessage::Devserver(devserver_msg) => match devserver_msg {
            DevserverMsg::HotReload(hotreload_message) => {
                dioxus_devtools::apply_changes(
                    &doc.vdom,
                    &hotreload_message,
                );
                for asset_path in &hotreload_message.assets {
                    if let Some(url) = asset_path.to_str() {
                        doc.inner
                            .borrow_mut()
                            .reload_resource_by_href(url);
                    }
                }
            }
            DevserverMsg::FullReloadStart => {}
            _ => {}
        },
        DioxusMessage::CreateHeadElement(el) => {
            doc.create_head_element(
                &el.name,
                &el.attributes,
                &el.contents,
            );
            doc.poll(Some(std::task::Context::from_waker(waker)));
        }
        DioxusMessage::ResourceLoad(resource) => {
            doc.inner.borrow_mut().load_resource(
                blitz_dom::net::ResourceLoadResponse {
                    request_id: 0,
                    node_id: None,
                    resolved_url: None,
                    result: Ok(resource.clone()),
                },
            );
        }
    }
}

/// Poll the VDOM until no more futures are ready, resolve animations,
/// paint the scene, and send it to the main thread.
///
/// Returns a fresh `Scene` for the next frame.
fn run_poll_and_paint(
    doc: &mut DioxusDocument,
    waker: &std::task::Waker,
    waker_flag: &AtomicBool,
    animation_time: f64,
    mut scene: Scene,
    result_tx: &Sender<VdomResult>,
) -> Scene {
    loop {
        waker_flag.store(false, Ordering::SeqCst);
        doc.poll(Some(std::task::Context::from_waker(waker)));
        if !waker_flag.load(Ordering::SeqCst) {
            break;
        }
    }

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
