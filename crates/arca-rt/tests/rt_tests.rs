use arca_rt::{ActorMailbox, CancellationToken, Channel, TaskScheduler};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[test]
fn test_work_stealing_scheduler() {
    let scheduler = TaskScheduler::new(4);
    let counter = Arc::new(AtomicU32::new(0));

    for i in 0..20 {
        let c = Arc::clone(&counter);
        scheduler.spawn(i, move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
    }

    let completed = scheduler.execute_work_stealing();
    assert_eq!(completed, 20);
    assert_eq!(counter.load(Ordering::SeqCst), 20);
}

#[test]
fn test_channels_and_actor_mailbox() {
    let ch = Channel::<i32>::new(Some(2));
    assert!(ch.send(10).is_ok());
    assert!(ch.send(20).is_ok());
    assert!(ch.send(30).is_err()); // Exceeds capacity

    assert_eq!(ch.recv(), Some(10));
    assert_eq!(ch.recv(), Some(20));
    assert_eq!(ch.recv(), None);

    let actor = ActorMailbox::<String>::new();
    actor.send("msg1".to_string()).unwrap();
    actor.send("msg2".to_string()).unwrap();

    let mut received = Vec::new();
    let processed = actor.process_all(|msg| received.push(msg));

    assert_eq!(processed, 2);
    assert_eq!(received, vec!["msg1", "msg2"]);
}

#[test]
fn test_cancellation_token() {
    let token = CancellationToken::new();
    assert!(!token.is_cancelled());
    token.cancel();
    assert!(token.is_cancelled());
}
