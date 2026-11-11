use super::*;
use crate::backup::Crossing;
use std::sync::mpsc::{RecvTimeoutError, TryRecvError};

#[test]
fn sending_an_order_is_not_completion() {
    let pending = Pending::default();
    let (ticket, reply) = pending
        .begin(5, Crossing::default().claim().unwrap())
        .unwrap();
    assert_eq!(reply.try_recv(), Err(TryRecvError::Empty));
    assert!(pending.finish(5, ticket, Ok(12)));
    assert_eq!(reply.recv().unwrap(), Ok(12));
}

#[test]
fn an_unfinished_clear_cannot_be_submitted_twice() {
    let pending = Pending::default();
    let (ticket, reply) = pending
        .begin(5, Crossing::default().claim().unwrap())
        .unwrap();
    assert_eq!(
        pending
            .begin(5, Crossing::default().claim().unwrap())
            .unwrap_err(),
        BUSY
    );
    assert!(pending.finish(5, ticket, Err(FAILED)));
    assert_eq!(reply.recv().unwrap(), Err(FAILED));
    assert!(
        pending
            .begin(5, Crossing::default().claim().unwrap())
            .is_ok()
    );
}

#[test]
fn late_answers_cannot_complete_another_clear() {
    let pending = Pending::default();
    let (first, reply) = pending
        .begin(5, Crossing::default().claim().unwrap())
        .unwrap();
    pending.stopped(5);
    assert_eq!(reply.recv().unwrap(), Err(STOPPED));
    let (second, reply) = pending
        .begin(6, Crossing::default().claim().unwrap())
        .unwrap();
    assert!(!pending.finish(5, first, Ok(1)));
    assert!(!pending.finish(6, first, Ok(1)));
    pending.stopped(5);
    assert_eq!(reply.try_recv(), Err(TryRecvError::Empty));
    assert!(pending.finish(6, second, Ok(2)));
    assert_eq!(reply.recv().unwrap(), Ok(2));
}

#[test]
fn a_timeout_does_not_let_another_destructive_order_overtake_the_first() {
    let pending = Pending::default();
    let crossing = Crossing::default();
    let (ticket, reply) = pending.begin(5, crossing.claim().unwrap()).unwrap();
    assert_eq!(
        reply.recv_timeout(Duration::ZERO),
        Err(RecvTimeoutError::Timeout)
    );
    drop(reply);
    assert!(crossing.underway());
    assert!(crossing.claim().is_none());
    assert_eq!(
        pending
            .begin(5, Crossing::default().claim().unwrap())
            .unwrap_err(),
        BUSY
    );
    assert!(pending.finish(5, ticket, Ok(1)));
    assert!(!crossing.underway());
    assert!(pending.begin(5, crossing.claim().unwrap()).is_ok());
}

#[test]
fn a_stopped_panel_releases_the_history_for_other_operations() {
    let pending = Pending::default();
    let crossing = Crossing::default();
    let (_, reply) = pending.begin(5, crossing.claim().unwrap()).unwrap();
    assert!(crossing.claim().is_none());
    pending.stopped(5);
    assert_eq!(reply.recv().unwrap(), Err(STOPPED));
    assert!(!crossing.underway());
    assert!(crossing.claim().is_some());
}

#[test]
fn an_explicit_restart_can_interrupt_only_a_timed_out_clear() {
    let pending = Pending::default();
    let crossing = Crossing::default();
    let (ticket, reply) = pending.begin(5, crossing.claim().unwrap()).unwrap();
    assert!(pending.interrupt_timed_out().is_none());
    pending.timed_out(6, ticket);
    assert!(pending.interrupt_timed_out().is_none());
    pending.timed_out(5, ticket);
    let restarting = pending
        .interrupt_timed_out()
        .expect("the user can stop the unresponsive panel");
    assert_eq!(reply.recv().unwrap(), Err(STOPPED));
    assert!(crossing.claim().is_none());
    assert!(!pending.finish(5, ticket, Ok(1)));
    drop(restarting);
    assert!(crossing.claim().is_some());
}

#[test]
fn restarting_cannot_take_the_lock_of_a_backup() {
    let pending = Pending::default();
    let crossing = Crossing::default();
    let _backup = crossing.claim().unwrap();
    assert!(crossing.claim().is_none());
    assert!(pending.interrupt_timed_out().is_none());
}

#[test]
fn only_complete_correlated_answers_are_accepted() {
    assert_eq!(answer("emptied 42 7"), Some((42, Ok(7))));
    assert_eq!(answer("empty-failed 42"), Some((42, Err(FAILED))));
    for invalid in [
        "emptied 7",
        "emptied 42 -1",
        "emptied no 7",
        "emptied 42 7 extra",
        "empty-failed 42 extra",
    ] {
        assert_eq!(answer(invalid), None, "{invalid}");
    }
}
