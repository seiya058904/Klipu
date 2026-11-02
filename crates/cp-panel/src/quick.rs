use crate::handing::{newest, paste_plain};
use crate::here;
use crate::note::note;
use cp_store::Store;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicIsize, Ordering};

pub struct Aim {
    pub store: Rc<Store>,
    pub engine: Option<Rc<crate::engine::Engine>>,
}

pub type Reach = Rc<dyn Fn() -> Aim>;

pub struct Quick {
    reach: Reach,
    ahead: Arc<AtomicIsize>,
}

impl Quick {
    pub fn new(reach: Reach, ahead: Arc<AtomicIsize>) -> Rc<Self> {
        Rc::new(Self { reach, ahead })
    }

    pub fn plain(self: &Rc<Self>) {
        let ahead = self.ahead.swap(0, Ordering::Relaxed);
        if ahead == 0 {
            note("nothing to paste plain into: no other window is in front");
            return;
        }
        let aim = (self.reach)();
        let Some((row, _)) = newest(&aim.store, 0) else {
            note("nothing to paste plain: the history is empty");
            return;
        };
        if let Err(refused) = paste_plain(&aim.store, aim.engine.as_deref(), row.id) {
            self.ahead.store(ahead, Ordering::Relaxed);
            note(&format!("the plain paste did not go through: {refused:?}"));
            return;
        }
        let later = Rc::downgrade(self);
        here::paste_into(
            ahead,
            || {},
            move |sent| match sent {
                here::Sent::Done => {}
                here::Sent::Nobody => note("no window was left to paste plain into"),
                here::Sent::Degraded(why) => {
                    note(&format!(
                        "pasted plain onto the clipboard, unpasted: {why:?}"
                    ));
                    if let Some(quick) = later.upgrade() {
                        quick.ahead.store(ahead, Ordering::Relaxed);
                    }
                }
            },
        );
    }
}
