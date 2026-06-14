//! Acceptance tests for the highest-risk SPEC-COMM-001 criteria.
//!
//! Each test names the acceptance criterion it covers (SPEC §Acceptance criteria).
//! The fixed clock lets lease-expiry and death-window tests run without sleeping.

use std::path::Path;
use std::sync::Arc;
use std::thread;

use mesh::clock::Clock;
use mesh::types::*;
use mesh::Daemon;

const T0: i64 = 1_700_000_000; // arbitrary fixed epoch anchor

fn daemon(dir: &Path, clock: Clock) -> Daemon {
    Daemon::open_with_clock(dir, clock).expect("open daemon")
}

fn reg(d: &Daemon, id: &str) {
    d.register(RegisterRequest {
        agent_id: id.into(),
        branch: format!("agent/{id}"),
        prompt_ptr: format!("agent/{id}:.machine-prompt"),
        role: None,
        ttl_seconds: Some(60),
    })
    .expect("register");
}

fn claim_req(id: &str, resource: &str, mode: ClaimMode, wait: WaitPolicy) -> ClaimRequest {
    ClaimRequest {
        agent_id: id.into(),
        resource: resource.into(),
        mode,
        lease_seconds: Some(100),
        wait,
        note: None,
    }
}

// --- #4: atomic exclusive grant under concurrent contention ----------------

#[test]
fn ac4_exactly_one_winner_under_contention() {
    let tmp = tempfile::tempdir().unwrap();
    // Two OS threads share one data dir via independent Daemon handles, racing
    // for the same exclusive resource. LMDB's process-shared lock arbitrates.
    let dir = tmp.path().to_path_buf();

    // Pre-create the env once so both threads attach to the same files.
    {
        let d = daemon(&dir, Clock::fixed_at(T0));
        reg(&d, "seed"); // ensures dir + dbs exist
    }

    let barrier = Arc::new(std::sync::Barrier::new(2));
    let mut handles = Vec::new();
    for id in ["a", "b"] {
        let dir = dir.clone();
        let barrier = barrier.clone();
        handles.push(thread::spawn(move || {
            let d = daemon(&dir, Clock::fixed_at(T0));
            barrier.wait();
            d.claim(claim_req(id, "module/x", ClaimMode::Exclusive, WaitPolicy::NoWait))
                .expect("claim")
        }));
    }
    let results: Vec<ClaimResponse> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    let granted: Vec<_> = results
        .iter()
        .filter(|r| r.status == ClaimStatus::Granted)
        .collect();
    let denied: Vec<_> = results
        .iter()
        .filter(|r| r.status == ClaimStatus::Denied)
        .collect();

    assert_eq!(granted.len(), 1, "exactly one grant: {results:?}");
    assert_eq!(denied.len(), 1, "the other must be denied: {results:?}");
    // Fence on the grant must be strictly positive (rose from 0).
    assert!(granted[0].fence >= 1, "fence must rise on grant");
}

#[test]
fn ac4_fence_strictly_increases_across_grants() {
    let tmp = tempfile::tempdir().unwrap();
    let d = daemon(tmp.path(), Clock::fixed_at(T0));
    reg(&d, "a");
    reg(&d, "b");

    let g1 = d
        .claim(claim_req("a", "r", ClaimMode::Exclusive, WaitPolicy::NoWait))
        .unwrap();
    assert_eq!(g1.status, ClaimStatus::Granted);
    d.release(ReleaseRequest {
        agent_id: "a".into(),
        claim_id: g1.claim_id.clone(),
        resource: "r".into(),
    })
    .unwrap();
    let g2 = d
        .claim(claim_req("b", "r", ClaimMode::Exclusive, WaitPolicy::NoWait))
        .unwrap();
    assert_eq!(g2.status, ClaimStatus::Granted);
    assert!(g2.fence > g1.fence, "fence must strictly increase: {} -> {}", g1.fence, g2.fence);
}

// --- #5: queue promotion on release ----------------------------------------

#[test]
fn ac5_queue_promotion_on_release() {
    let tmp = tempfile::tempdir().unwrap();
    let d = daemon(tmp.path(), Clock::fixed_at(T0));
    reg(&d, "a");
    reg(&d, "b");

    let g = d
        .claim(claim_req("a", "r", ClaimMode::Exclusive, WaitPolicy::NoWait))
        .unwrap();
    assert_eq!(g.status, ClaimStatus::Granted);

    let q = d
        .claim(claim_req("b", "r", ClaimMode::Exclusive, WaitPolicy::Queue))
        .unwrap();
    assert_eq!(q.status, ClaimStatus::Queued);
    assert_eq!(q.queue_position, Some(1));

    let rel = d
        .release(ReleaseRequest {
            agent_id: "a".into(),
            claim_id: g.claim_id,
            resource: "r".into(),
        })
        .unwrap();
    assert_eq!(rel.status, ReleaseStatus::Released);
    assert_eq!(rel.next_holder.as_deref(), Some("b"), "b promoted");
    assert!(rel.fence.unwrap() > g.fence, "fence increments on promotion");

    // b now holds r.
    let view = d
        .claims(ClaimsRequest {
            agent_id: "a".into(),
            resource: Some("r".into()),
        })
        .unwrap();
    assert_eq!(view.claims.len(), 1);
    assert_eq!(view.claims[0].holder, vec!["b".to_string()]);
}

// --- #6: lease expiry self-heals a dead holder's lock ----------------------

#[test]
fn ac6_lease_expiry_self_heals() {
    let tmp = tempfile::tempdir().unwrap();
    let clock = Clock::fixed_at(T0);
    let d = daemon(tmp.path(), clock.clone());
    reg(&d, "a");
    reg(&d, "b");

    // a takes a short lease, then "crashes" (never releases).
    let g = d
        .claim(ClaimRequest {
            agent_id: "a".into(),
            resource: "r".into(),
            mode: ClaimMode::Exclusive,
            lease_seconds: Some(10),
            wait: WaitPolicy::NoWait,
            note: None,
        })
        .unwrap();
    assert_eq!(g.status, ClaimStatus::Granted);

    // b queues behind a.
    let q = d
        .claim(claim_req("b", "r", ClaimMode::Exclusive, WaitPolicy::Queue))
        .unwrap();
    assert_eq!(q.status, ClaimStatus::Queued);

    // Advance past a's lease. The next claim/sweep frees a and promotes b.
    clock.advance(20);
    d.sweep(clock.now_unix()).unwrap();

    let view = d
        .claims(ClaimsRequest {
            agent_id: "b".into(),
            resource: Some("r".into()),
        })
        .unwrap();
    assert_eq!(view.claims.len(), 1, "lock must remain (held by promoted b)");
    assert_eq!(view.claims[0].holder, vec!["b".to_string()], "b promoted after a's lease expired");
    assert!(view.claims[0].fence > g.fence);
}

#[test]
fn ac11_dead_agent_claims_released() {
    // #11: liveness from heartbeat; a dead agent's claims are released even with
    // a long lease still nominally valid.
    let tmp = tempfile::tempdir().unwrap();
    let clock = Clock::fixed_at(T0);
    let d = daemon(tmp.path(), clock.clone());
    reg(&d, "a"); // ttl 60
    let g = d
        .claim(ClaimRequest {
            agent_id: "a".into(),
            resource: "r".into(),
            mode: ClaimMode::Exclusive,
            lease_seconds: Some(100_000), // lease far in the future
            wait: WaitPolicy::NoWait,
            note: None,
        })
        .unwrap();
    assert_eq!(g.status, ClaimStatus::Granted);

    // Advance past ttl(60) + grace(30) so a is dead by heartbeat, though its
    // lease is still valid. Death cleanup must still free the lock.
    clock.advance(200);
    d.sweep(clock.now_unix()).unwrap();

    let view = d
        .claims(ClaimsRequest {
            agent_id: "a".into(),
            resource: None,
        })
        .unwrap();
    assert!(view.claims.is_empty(), "dead agent's claim must be released: {view:?}");
}

// --- #7: non-holder cannot release -----------------------------------------

#[test]
fn ac7_non_holder_cannot_release() {
    let tmp = tempfile::tempdir().unwrap();
    let d = daemon(tmp.path(), Clock::fixed_at(T0));
    reg(&d, "a");
    reg(&d, "b");

    let g = d
        .claim(claim_req("a", "r", ClaimMode::Exclusive, WaitPolicy::NoWait))
        .unwrap();

    // b tries to release a's claim_id.
    let bad = d
        .release(ReleaseRequest {
            agent_id: "b".into(),
            claim_id: g.claim_id.clone(),
            resource: "r".into(),
        })
        .unwrap();
    assert_eq!(bad.status, ReleaseStatus::NotHolder);

    // a still holds r.
    let view = d
        .claims(ClaimsRequest {
            agent_id: "a".into(),
            resource: Some("r".into()),
        })
        .unwrap();
    assert_eq!(view.claims[0].holder, vec!["a".to_string()]);
}

// --- #8: durable mail survives sender death, exactly-once via cursor --------

#[test]
fn ac8_durable_mail_survives_sender_death_exactly_once() {
    let tmp = tempfile::tempdir().unwrap();
    let clock = Clock::fixed_at(T0);

    let post_id = {
        // a posts to b, then "dies" (handle dropped).
        let d = daemon(tmp.path(), clock.clone());
        reg(&d, "a");
        let p = d
            .post(PostRequest {
                agent_id: "a".into(),
                to: "b".into(),
                subject: Some("hi".into()),
                body: "ping".into(),
                reply_to: None,
                ttl_seconds: None,
            })
            .unwrap();
        assert_eq!(p.fanout, 1);
        p.message_id
    };

    // New daemon handle (simulating fresh process); b did not exist at post time.
    let d = daemon(tmp.path(), clock.clone());
    reg(&d, "b");
    let inbox1 = d
        .inbox(InboxRequest {
            agent_id: "b".into(),
            since: None,
            topics: vec![],
            limit: None,
        })
        .unwrap();
    assert_eq!(inbox1.messages.len(), 1, "b receives a's durable message");
    assert_eq!(inbox1.messages[0].message_id, post_id);
    assert_eq!(inbox1.messages[0].body, "ping");

    // Acknowledge, then re-poll: must not re-deliver (exactly once).
    d.read(ReadRequest {
        agent_id: "b".into(),
        up_to: post_id.clone(),
    })
    .unwrap();
    let inbox2 = d
        .inbox(InboxRequest {
            agent_id: "b".into(),
            since: None,
            topics: vec![],
            limit: None,
        })
        .unwrap();
    assert!(inbox2.messages.is_empty(), "read advanced cursor; no re-delivery");
}

// --- #9: broadcast stored once, read per cursor ----------------------------

#[test]
fn ac9_broadcast_stored_once_read_per_cursor() {
    let tmp = tempfile::tempdir().unwrap();
    let d = daemon(tmp.path(), Clock::fixed_at(T0));
    for id in ["a", "b", "c"] {
        reg(&d, id);
    }
    let p = d
        .post(PostRequest {
            agent_id: "a".into(),
            to: "*".into(),
            subject: None,
            body: "all-hands".into(),
            reply_to: None,
            ttl_seconds: None,
        })
        .unwrap();
    assert_eq!(p.fanout, 3, "fanout counts known agents");

    // Each recipient sees it exactly once via its own cursor.
    for id in ["b", "c"] {
        let inbox = d
            .inbox(InboxRequest {
                agent_id: id.into(),
                since: None,
                topics: vec![],
                limit: None,
            })
            .unwrap();
        assert_eq!(inbox.messages.len(), 1, "{id} sees broadcast once");
        assert_eq!(inbox.messages[0].message_id, p.message_id);
    }
}

// --- #10: topic readable by a late joiner ----------------------------------

#[test]
fn ac10_topic_readable_by_late_joiner() {
    let tmp = tempfile::tempdir().unwrap();
    let d = daemon(tmp.path(), Clock::fixed_at(T0));
    reg(&d, "a");
    // Post to topic:build before c exists.
    let p = d
        .post(PostRequest {
            agent_id: "a".into(),
            to: "topic:build".into(),
            subject: None,
            body: "build green".into(),
            reply_to: None,
            ttl_seconds: None,
        })
        .unwrap();

    // c joins later and reads the topic from its own cursor.
    reg(&d, "c");
    let inbox = d
        .inbox(InboxRequest {
            agent_id: "c".into(),
            since: None,
            topics: vec!["build".into()],
            limit: None,
        })
        .unwrap();
    assert_eq!(inbox.messages.len(), 1, "late joiner reads pending topic msg");
    assert_eq!(inbox.messages[0].message_id, p.message_id);

    // Without subscribing, c sees nothing on that topic.
    let none = d
        .inbox(InboxRequest {
            agent_id: "c".into(),
            since: None,
            topics: vec![],
            limit: None,
        })
        .unwrap();
    assert!(none.messages.is_empty(), "no topic subscription => no topic mail");
}

// --- #12: privacy of mail and cursors --------------------------------------

#[test]
fn ac12_mail_and_cursor_privacy() {
    let tmp = tempfile::tempdir().unwrap();
    let d = daemon(tmp.path(), Clock::fixed_at(T0));
    reg(&d, "a");
    reg(&d, "b");

    // a posts a private message to b.
    let p = d
        .post(PostRequest {
            agent_id: "a".into(),
            to: "b".into(),
            subject: None,
            body: "secret".into(),
            reply_to: None,
            ttl_seconds: None,
        })
        .unwrap();

    // a (a third party here w.r.t. b's mailbox) must NOT see b's point-to-point mail.
    let a_inbox = d
        .inbox(InboxRequest {
            agent_id: "a".into(),
            since: None,
            topics: vec![],
            limit: None,
        })
        .unwrap();
    assert!(
        a_inbox.messages.iter().all(|m| m.message_id != p.message_id),
        "a cannot read b's private mail"
    );

    // b advancing its own cursor must not affect a's cursor.
    d.read(ReadRequest {
        agent_id: "b".into(),
        up_to: p.message_id.clone(),
    })
    .unwrap();

    // Now a sends to a-topic both subscribe to; ensure b reading did not advance a.
    let broadcast = d
        .post(PostRequest {
            agent_id: "b".into(),
            to: "*".into(),
            subject: None,
            body: "hello".into(),
            reply_to: None,
            ttl_seconds: None,
        })
        .unwrap();
    let a_inbox2 = d
        .inbox(InboxRequest {
            agent_id: "a".into(),
            since: None,
            topics: vec![],
            limit: None,
        })
        .unwrap();
    assert!(
        a_inbox2.messages.iter().any(|m| m.message_id == broadcast.message_id),
        "a's own cursor is independent and still sees the broadcast"
    );
}

// --- #3: exactly eight verbs ----------------------------------------------

#[test]
fn ac3_exactly_eight_verbs() {
    use mesh::mcp::VERBS;
    assert_eq!(VERBS.len(), 8);
    let expected = [
        "register", "roster", "claim", "release", "claims", "post", "inbox", "read",
    ];
    assert_eq!(VERBS, expected, "tool list must be exactly these eight");
}

// --- #14: storage shape (LMDB primary + SQLite-WAL journal) -----------------

#[test]
fn ac14_storage_shape_on_disk() {
    let tmp = tempfile::tempdir().unwrap();
    let d = daemon(tmp.path(), Clock::fixed_at(T0));
    reg(&d, "a");
    d.post(PostRequest {
        agent_id: "a".into(),
        to: "*".into(),
        subject: None,
        body: "x".into(),
        reply_to: None,
        ttl_seconds: None,
    })
    .unwrap();
    drop(d);

    let dir = tmp.path();
    assert!(dir.join("data.mdb").exists(), "LMDB primary present");
    assert!(dir.join("lock.mdb").exists(), "LMDB lock present");
    assert!(
        dir.join("journal/history.db").exists(),
        "SQLite journal present"
    );
}

// --- #2/§4.2 shared mode co-holding ---------------------------------------

#[test]
fn shared_mode_allows_co_holders_but_blocks_exclusive() {
    let tmp = tempfile::tempdir().unwrap();
    let d = daemon(tmp.path(), Clock::fixed_at(T0));
    for id in ["a", "b", "c"] {
        reg(&d, id);
    }
    let g1 = d
        .claim(claim_req("a", "r", ClaimMode::Shared, WaitPolicy::NoWait))
        .unwrap();
    assert_eq!(g1.status, ClaimStatus::Granted);
    let g2 = d
        .claim(claim_req("b", "r", ClaimMode::Shared, WaitPolicy::NoWait))
        .unwrap();
    assert_eq!(g2.status, ClaimStatus::Granted, "shared co-holder allowed");

    // An exclusive request must not be granted while shared holders exist.
    let ex = d
        .claim(claim_req("c", "r", ClaimMode::Exclusive, WaitPolicy::NoWait))
        .unwrap();
    assert_eq!(ex.status, ClaimStatus::Denied);

    let view = d
        .claims(ClaimsRequest {
            agent_id: "a".into(),
            resource: Some("r".into()),
        })
        .unwrap();
    assert_eq!(view.claims[0].holder.len(), 2, "two shared holders");
}

// --- idempotent holder renewal (C-14/C-15) ---------------------------------

#[test]
fn holder_renewal_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let clock = Clock::fixed_at(T0);
    let d = daemon(tmp.path(), clock.clone());
    reg(&d, "a");
    let g1 = d
        .claim(claim_req("a", "r", ClaimMode::Exclusive, WaitPolicy::NoWait))
        .unwrap();
    let g2 = d
        .claim(claim_req("a", "r", ClaimMode::Exclusive, WaitPolicy::NoWait))
        .unwrap();
    assert_eq!(g2.status, ClaimStatus::Granted);
    assert_eq!(g1.claim_id, g2.claim_id, "same grant returned");
    assert_eq!(g1.fence, g2.fence, "renewal does not bump fence (C-14)");
}
