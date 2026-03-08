use stellarconduit_core::discovery::{events::DiscoveryEvent, peer_list::PeerList};

// ──── insert_or_update ───────────────────────────────────────────────────────

#[test]
fn test_new_peer_returns_peer_discovered() {
    let mut list = PeerList::new(60);
    let pubkey = [1u8; 32];
    let event = list.insert_or_update(pubkey, 80).unwrap();
    assert!(matches!(event, DiscoveryEvent::PeerDiscovered(_)));
}

#[test]
fn test_known_peer_returns_peer_updated() {
    let mut list = PeerList::new(60);
    let pubkey = [2u8; 32];
    list.insert_or_update(pubkey, 70); // first insert
    let event = list.insert_or_update(pubkey, 75).unwrap(); // second = update
    assert!(matches!(event, DiscoveryEvent::PeerUpdated(_, 75)));
}

#[test]
fn test_signal_strength_passed_through_in_update() {
    let mut list = PeerList::new(60);
    let pubkey = [3u8; 32];
    list.insert_or_update(pubkey, 50);
    if let Some(DiscoveryEvent::PeerUpdated(_, rssi)) = list.insert_or_update(pubkey, 99) {
        assert_eq!(rssi, 99);
    } else {
        panic!("Expected PeerUpdated");
    }
}

// ──── get_active_peers ───────────────────────────────────────────────────────

#[test]
fn test_active_peers_returns_fresh_peers() {
    let mut list = PeerList::new(60);
    list.insert_or_update([10u8; 32], 80);
    list.insert_or_update([11u8; 32], 60);
    assert_eq!(list.get_active_peers().len(), 2);
}

#[test]
fn test_stale_peers_not_returned_by_get_active_peers() {
    let mut list = PeerList::new(30);
    let pubkey = [20u8; 32];
    list.insert_or_update(pubkey, 80);

    // Manually backdate last_seen to simulate expiry
    let old_ts = 0u64; // very old timestamp
    list.set_last_seen(&pubkey, old_ts);

    assert_eq!(
        list.get_active_peers().len(),
        0,
        "stale peer should not appear in active list"
    );
}

// ──── prune_stale_peers ──────────────────────────────────────────────────────

#[test]
fn test_prune_removes_stale_peers() {
    let mut list = PeerList::new(30);
    let pubkey = [30u8; 32];
    list.insert_or_update(pubkey, 80);
    list.set_last_seen(&pubkey, 0); // force stale

    let events = list.prune_stale_peers();
    assert_eq!(events.len(), 1, "should have one PeerLost event");
    assert!(matches!(events[0], DiscoveryEvent::PeerLost(_)));
    assert_eq!(list.len(), 0, "stale peer should be removed");
}

#[test]
fn test_prune_keeps_fresh_peers() {
    let mut list = PeerList::new(60);
    list.insert_or_update([40u8; 32], 80); // fresh
    let stale = [41u8; 32];
    list.insert_or_update(stale, 80);
    list.set_last_seen(&stale, 0); // force stale

    let events = list.prune_stale_peers();
    assert_eq!(events.len(), 1, "only one peer should be pruned");
    assert_eq!(list.len(), 1, "fresh peer should remain");
}

#[test]
fn test_prune_returns_empty_when_all_fresh() {
    let mut list = PeerList::new(60);
    list.insert_or_update([50u8; 32], 80);
    list.insert_or_update([51u8; 32], 60);

    let events = list.prune_stale_peers();
    assert!(events.is_empty(), "no events when no stale peers");
}

#[test]
fn test_prune_emits_peer_lost_with_correct_identity() {
    let mut list = PeerList::new(30);
    let pubkey = [60u8; 32];
    list.insert_or_update(pubkey, 80);
    list.set_last_seen(&pubkey, 0);

    let events = list.prune_stale_peers();
    if let DiscoveryEvent::PeerLost(identity) = &events[0] {
        assert_eq!(identity.pubkey, pubkey);
    } else {
        panic!("Expected PeerLost event");
    }
}

// ──── general ────────────────────────────────────────────────────────────────

#[test]
fn test_empty_list_is_empty() {
    let list = PeerList::new(60);
    assert!(list.is_empty());
    assert_eq!(list.len(), 0);
}

#[test]
fn test_insert_increments_len() {
    let mut list = PeerList::new(60);
    list.insert_or_update([70u8; 32], 80);
    assert_eq!(list.len(), 1);
    list.insert_or_update([71u8; 32], 80);
    assert_eq!(list.len(), 2);
    // Updating existing peer should not increment
    list.insert_or_update([70u8; 32], 50);
    assert_eq!(list.len(), 2);
}
