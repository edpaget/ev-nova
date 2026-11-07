//! The loop events' channel: how the menu bar, the Apple events and
//! WinSparkle send the app a [`LoopEvent`] through the event loop.
//!
//! winit 0.31's proxy carries no data: [`EventLoopProxy::wake_up`] only
//! wakes the loop, which calls `ApplicationHandler::proxy_wake_up`.
//! Wake-ups coalesce, and on Windows one may be lost under contention
//! (winit#3687). So each event goes into a channel the game owns
//! ([`LoopSender::send`]), and the runner drains it ([`LoopEvents::drain`])
//! on each wake-up and again before the loop waits, so an event whose
//! wake-up was lost is taken all the same.

use std::sync::mpsc::{Receiver, Sender, channel};

use winit::event_loop::EventLoopProxy;

use crate::app::LoopEvent;

/// The channel's two ends: the sender for each thread or handler that
/// sends, cloned for each, and the events the runner drains, over the
/// loop `proxy` wakes.
#[must_use]
pub fn loop_channel(proxy: EventLoopProxy) -> (LoopSender, LoopEvents) {
    let (events, received) = channel();
    (LoopSender { events, proxy }, LoopEvents(received))
}

/// Sends the app [`LoopEvent`]s through the event loop, from any thread.
#[derive(Clone, Debug)]
pub struct LoopSender {
    events: Sender<LoopEvent>,
    proxy: EventLoopProxy,
}

impl LoopSender {
    /// Queues `event` for the app and wakes the loop. Once the loop is
    /// gone (the game is exiting) the event is dropped, and nothing is
    /// woken.
    pub fn send(&self, event: LoopEvent) {
        if self.events.send(event).is_ok() {
            self.proxy.wake_up();
        }
    }
}

/// The loop events sent and not yet taken, for the runner.
#[derive(Debug)]
pub struct LoopEvents(Receiver<LoopEvent>);

impl LoopEvents {
    /// Every event waiting, in the order it was sent.
    pub fn drain(&self) -> impl Iterator<Item = LoopEvent> + '_ {
        self.0.try_iter()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use nova_view::MenuCommand;
    use winit::event_loop::{EventLoopProxy, EventLoopProxyProvider};

    use super::*;
    use crate::app::MenuPick;

    /// A loop that counts how often it is woken.
    #[derive(Debug, Default)]
    struct CountingLoop(AtomicUsize);

    impl EventLoopProxyProvider for CountingLoop {
        fn wake_up(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl CountingLoop {
        fn wakes(&self) -> usize {
            self.0.load(Ordering::SeqCst)
        }
    }

    fn channel() -> (Arc<CountingLoop>, LoopSender, LoopEvents) {
        let counting = Arc::new(CountingLoop::default());
        let proxy = EventLoopProxy::new(Arc::clone(&counting) as Arc<dyn EventLoopProxyProvider>);
        let (sender, events) = loop_channel(proxy);
        (counting, sender, events)
    }

    fn quit() -> LoopEvent {
        LoopEvent::Menu(MenuPick::Command(MenuCommand::Quit))
    }

    #[test]
    fn a_sent_event_waits_and_wakes_the_loop() {
        let (counting, sender, events) = channel();
        assert_eq!(counting.wakes(), 0);
        sender.send(LoopEvent::UpdaterQuit);
        assert_eq!(counting.wakes(), 1);
        sender.clone().send(quit());
        assert_eq!(counting.wakes(), 2, "one wake-up a send");
        assert_eq!(events.drain().count(), 2);
    }

    #[test]
    fn events_drain_in_the_order_sent() {
        let (_, sender, events) = channel();
        sender.send(quit());
        sender.send(LoopEvent::UpdaterQuit);
        assert_eq!(
            events.drain().collect::<Vec<_>>(),
            [quit(), LoopEvent::UpdaterQuit]
        );
        assert_eq!(events.drain().count(), 0, "drained");
        sender.send(LoopEvent::UpdaterQuit);
        assert_eq!(events.drain().collect::<Vec<_>>(), [LoopEvent::UpdaterQuit]);
    }

    #[test]
    fn a_send_from_another_thread_arrives() {
        let (counting, sender, events) = channel();
        std::thread::spawn(move || sender.send(LoopEvent::UpdaterQuit))
            .join()
            .expect("sent");
        assert_eq!(counting.wakes(), 1);
        assert_eq!(events.drain().collect::<Vec<_>>(), [LoopEvent::UpdaterQuit]);
    }

    #[test]
    fn a_send_after_the_loop_is_gone_wakes_nothing() {
        let (counting, sender, events) = channel();
        drop(events);
        sender.send(LoopEvent::UpdaterQuit);
        assert_eq!(counting.wakes(), 0);
    }
}
