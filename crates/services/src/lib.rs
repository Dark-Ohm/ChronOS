//! System-integration services for Chronos (GPUI-agnostic).
//!
//! Each service is a subscriber holding a `futures_signals::Mutable<T>` and
//! implements the lightweight `Service` trait. Commands are concrete methods
//! on each subscriber (NOT part of the trait).

pub mod compositor;

pub use compositor::{
    ActiveWindow, CompositorBackend, CompositorCommand, CompositorState, CompositorSubscriber,
    Monitor, Workspace,
};

use futures_signals::signal::Signal;

/// Availability of a service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceStatus {
    /// Created; first connection attempt pending (set by `new()`).
    Initializing,
    /// Fully functional.
    Available,
    /// All connection attempts failed; retry loop running in background.
    Unavailable,
    /// Connected but some features missing (e.g. NM present, Wi-Fi hardware absent).
    Degraded(String),
}

/// Lightweight, unified service contract: availability + reactivity.
/// Commands are concrete methods on each subscriber, not part of this trait.
pub trait Service: Send + Sync + 'static {
    type Data: Clone + 'static;
    type Error: Send + Sync + 'static;
    /// Reactive signal. Hides the `Mutable`; consumer cannot call `.set()`.
    fn subscribe(&self) -> impl Signal<Item = Self::Data> + Unpin + 'static;
    fn get(&self) -> Self::Data;
    fn status(&self) -> ServiceStatus;
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_signals::signal::Mutable;

    struct FakeService {
        data: Mutable<u32>,
    }

    impl Service for FakeService {
        type Data = u32;
        type Error = anyhow::Error;
        fn subscribe(&self) -> impl Signal<Item = u32> + Unpin + 'static {
            self.data.signal_cloned()
        }
        fn get(&self) -> u32 {
            self.data.get()
        }
        fn status(&self) -> ServiceStatus {
            ServiceStatus::Available
        }
    }

    #[tokio::test]
    async fn service_contract_emits_on_mutate() {
        use futures_signals::signal::SignalExt;
        use futures_util::stream::StreamExt;
        let svc = FakeService {
            data: Mutable::new(1),
        };
        let sig = svc.subscribe();
        assert_eq!(svc.get(), 1);
        assert_eq!(svc.status(), ServiceStatus::Available);

        svc.data.set(42);
        let mut stream = sig.to_stream();
        let received = stream.next().await.expect("signal emits");
        assert_eq!(received, 42);
    }

    #[test]
    fn anyhow_error_satisfies_error_bound() {
        fn takes_error<E: Send + Sync + 'static>(_e: E) {}
        let err = anyhow::Error::msg("test");
        takes_error(err); // anyhow::Error satisfies Send + Sync + 'static
    }
}
