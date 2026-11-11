use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

pub const WAIT: Duration = Duration::from_secs(30);
pub const BUSY: &str = "empty-busy";
pub const STOPPED: &str = "empty-stopped";
pub const FAILED: &str = "empty-failed";
pub const TIMEOUT: &str = "empty-timeout";
pub type Outcome = Result<i64, &'static str>;

struct Request {
    panel: u32,
    ticket: u64,
    reply: Sender<Outcome>,
    _exclusive: crate::backup::Alone,
    timed_out: bool,
}

#[derive(Default)]
pub struct Pending {
    next: AtomicU64,
    request: Mutex<Option<Request>>,
}

impl Pending {
    pub fn begin(
        &self,
        panel: u32,
        exclusive: crate::backup::Alone,
    ) -> Result<(u64, Receiver<Outcome>), &'static str> {
        let mut pending = self.request.lock().map_err(|_| STOPPED)?;
        if pending.is_some() {
            return Err(BUSY);
        }
        let ticket = self.next.fetch_add(1, Ordering::Relaxed);
        let (reply, receive) = channel();
        *pending = Some(Request {
            panel,
            ticket,
            reply,
            _exclusive: exclusive,
            timed_out: false,
        });
        Ok((ticket, receive))
    }

    pub fn finish(&self, panel: u32, ticket: u64, outcome: Outcome) -> bool {
        let Ok(mut pending) = self.request.lock() else {
            return false;
        };
        if !pending
            .as_ref()
            .is_some_and(|one| one.panel == panel && one.ticket == ticket)
        {
            return false;
        }
        if let Some(one) = pending.take() {
            let _ = one.reply.send(outcome);
        }
        true
    }

    pub fn stopped(&self, panel: u32) {
        if let Ok(mut pending) = self.request.lock()
            && pending.as_ref().is_some_and(|one| one.panel == panel)
            && let Some(one) = pending.take()
        {
            let _ = one.reply.send(Err(STOPPED));
        }
    }

    pub fn timed_out(&self, panel: u32, ticket: u64) {
        if let Ok(mut pending) = self.request.lock()
            && let Some(one) = pending.as_mut()
            && one.panel == panel
            && one.ticket == ticket
        {
            one.timed_out = true;
        }
    }

    pub fn interrupt_timed_out(&self) -> Option<crate::backup::Alone> {
        let mut pending = self.request.lock().ok()?;
        if !pending.as_ref().is_some_and(|one| one.timed_out) {
            return None;
        }
        let one = pending.take()?;
        let _ = one.reply.send(Err(STOPPED));
        Some(one._exclusive)
    }
}

pub fn answer(said: &str) -> Option<(u64, Outcome)> {
    let mut words = said.split_whitespace();
    let verb = words.next()?;
    let ticket = words.next()?.parse().ok()?;
    let outcome = match verb {
        "emptied" => {
            let gone: i64 = words.next()?.parse().ok()?;
            if gone < 0 {
                return None;
            }
            Ok(gone)
        }
        "empty-failed" => Err(FAILED),
        _ => return None,
    };
    words.next().is_none().then_some((ticket, outcome))
}

#[cfg(test)]
#[path = "emptying_test.rs"]
mod tests;
