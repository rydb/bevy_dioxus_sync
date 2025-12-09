use std::{
    any::type_name,
    collections::HashSet,
    fmt::{Debug, Display},
    sync::{
        Arc, Mutex, OnceLock,
    },
};

use arc_swap::ArcSwap;
use bevy_log::warn;
use dioxus_core::ReactiveContext;

#[derive(Clone, Debug)]
pub enum SignalFetchError {
    Uninitialized,
}

impl Display for SignalFetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error_stirng = match self {
            SignalFetchError::Uninitialized => "signal uninitialized",
        };
        write!(f, "{}", error_stirng)
    }
}

// pub enum CrossDomSignal<T> {
//     /// Uninitialized signal. atomic pointer to a Arcswap's pointer so this can be updated by bevy without polling by dioxus
//     Request(OnceCell<Arc<ArcSwap<T>>>),
//     Initialized(Arc<ArcSwap<T>>)
// }

/// A sync signal that works across vdoms.
pub struct CrossDomSignal<T> {
    value: Arc<OnceLock<Arc<ArcSwap<T>>>>,
    subscribers: Arc<Mutex<HashSet<ReactiveContext>>>,
}

impl<T> Default for CrossDomSignal<T> {
    fn default() -> Self {
        Self {
            value: Arc::new(OnceLock::new()),
            subscribers: Default::default(),
        }
    }
}

impl<T> Clone for CrossDomSignal<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            subscribers: self.subscribers.clone(),
        }
    }
}

impl<T: Display> Display for CrossDomSignal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display_string = match self.get() {
            Ok(n) => n.to_string(),
            Err(err) => err.to_string(),
        };
        write!(f, "{}", display_string)
    }
}

// impl<T: Debug> Debug for CrossDomSignal<T> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let subscribers_string = self.subscribers.try_lock().map(|n| n.iter().map(|n| n.to_string()).collect::<String>()).unwrap_or("???".to_string());
//         f.debug_struct("CrossDomSignal").field("value", &self.value).field("subscribers", &subscribers_string)
//         .finish()
//     }
// }

/// errors for setting signal values
#[derive(Debug)]
pub enum SetError {
    Poisoned(String),
    SignalFetchError(SignalFetchError),
    Blocking(String),
}

impl Display for SetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let erorr = match self {
            SetError::Poisoned(err) => format!("lock poisoned: {}", err),
            SetError::SignalFetchError(signal_fetch_error) => {
                format!("signal fetch error: {}", signal_fetch_error)
            }
            SetError::Blocking(err) => format!("set attempt would be blocking: {}", err),
        };
        write!(f, "{}", erorr)
    }
}

impl<T> CrossDomSignal<T> {
    pub fn get(&self) -> Result<Arc<T>, SignalFetchError> {
        // Subscribe the context observing the signal (if any) to updates of its value.
        match ReactiveContext::current() {
            Some(reactive_context) => {
                // panic!("reactive contxt found: {}", reactive_context.to_string());
                reactive_context.subscribe(self.subscribers.clone());
            }
            None => {
                // panic!("no reactive context found?");
            }
        }
        match self.value.get() {
            Some(value) => Ok(value.as_ref().load().clone()),
            None => Err(SignalFetchError::Uninitialized),
        }
    }

    pub fn set<'a>(&self, value: T) -> Result<(), SetError> {
        // Update the state
        let arc = match self.value.get() {
            Some(arc) => arc,
            None => return Err(SetError::SignalFetchError(SignalFetchError::Uninitialized)),
        };

        arc.store(value.into());
        // Trigger a re-render of the components that observed the signal's previous value
        let mut subscribers = {
            // Create a scope for the guard
            let mut lock = match self.subscribers.try_lock() {
                Ok(lock) => lock,
                Err(err) => return Err(SetError::Poisoned(err.to_string())),
            };
            std::mem::take(&mut *lock)
        };
        // println!("reactive context: {:#?}", subscribers.iter().map(|n| n.origin_scope()).collect::<Vec<_>>());

        subscribers.retain(|reactive_context| reactive_context.mark_dirty());
        // Extend the subscribers list instead of overwriting it in case a subscriber is added while reactive contexts are marked dirty

        match self.subscribers.try_lock() {
            Ok(mut lock) => lock.extend(subscribers),
            Err(err) => {
                warn!("{}", err);
                return Err(SetError::Blocking(err.to_string()));
            }
        }
        // warn!("successfully set value!");
        Ok(())
    }
    pub fn new(value: T) -> Self {
        let once = OnceLock::new();
        let _ = once
            .set(Arc::new(ArcSwap::new(value.into())))
            .inspect_err(|_err| {
                warn!(
                    "Initializing OnceLock failed for {}. How did this happen?",
                    type_name::<T>()
                )
            });
        Self {
            value: Arc::new(once),
            subscribers: Default::default(),
        }
    }
    pub fn new_uninitialized() -> Self {
        Self {
            value: Arc::new(OnceLock::new()),
            subscribers: Default::default(),
        }
    }
    pub fn pnt_to(&self, ptr: Arc<ArcSwap<T>>) {
        let _ = self
            .value
            .set(ptr)
            .inspect_err(|_err| warn!("initialize failed for {}", type_name::<T>()));
    }
    pub fn initialize(&self, value: T) {
        let _ = self
            .value
            .set(ArcSwap::new(value.into()).into())
            .inspect_err(|_err| warn!("initialize failed for {}", type_name::<T>()));
    }
    /// gets the ptr to the value's pointer
    pub fn get_ptr(&self) -> Option<Arc<ArcSwap<T>>> {
        self.value.get().cloned()
    }
}
