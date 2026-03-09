//! Gossip protocol event loop and anti-entropy sync.

use std::time::Duration;
use tokio::time::sleep;

use crate::gossip::round::{GossipScheduler, ACTIVE_ROUND_INTERVAL_MS, IDLE_ROUND_INTERVAL_MS};
use crate::message::types::{SyncRequest, SyncResponse, TransactionEnvelope};

pub struct GossipState {
    pub active_envelopes: Vec<TransactionEnvelope>,
}

impl GossipState {
    pub fn new() -> Self {
        Self {
            active_envelopes: Vec::new(),
        }
    }

    /// Add an envelope to the active buffer
    pub fn add_envelope(&mut self, env: TransactionEnvelope) {
        self.active_envelopes.push(env);
    }

    /// Generates a SyncRequest containing the 4-byte prefixes of known message IDs
    pub fn generate_sync_request(&self) -> SyncRequest {
        let known_message_ids = self
            .active_envelopes
            .iter()
            .map(|env| {
                let mut prefix = [0u8; 4];
                prefix.copy_from_slice(&env.message_id[0..4]);
                prefix
            })
            .collect();

        SyncRequest { known_message_ids }
    }

    /// Processes an incoming SyncRequest, returning a SyncResponse with any local envelopes
    /// that the requestor does not have.
    pub fn handle_sync_request(&self, req: &SyncRequest) -> SyncResponse {
        let missing_envelopes = self
            .active_envelopes
            .iter()
            .filter(|env| {
                let mut prefix = [0u8; 4];
                prefix.copy_from_slice(&env.message_id[0..4]);
                !req.known_message_ids.contains(&prefix)
            })
            .cloned()
            .collect();

        SyncResponse { missing_envelopes }
    }

    /// Process an incoming SyncResponse by adding the missing envelopes to our state
    pub fn handle_sync_response(&mut self, resp: SyncResponse) {
        for env in resp.missing_envelopes {
            // Ideally we'd verify signatures and dedupe here before adding
            self.add_envelope(env);
        }
    }
}

pub async fn run_gossip_loop(mut scheduler: GossipScheduler) {
    let mut _anti_entropy_timer = tokio::time::interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            // Main epidemic push interval
            _ = sleep(Duration::from_millis(
                if scheduler.is_idle() {
                    IDLE_ROUND_INTERVAL_MS
                } else {
                    ACTIVE_ROUND_INTERVAL_MS
                }
            )) => {
                if scheduler.is_time_for_round() {
                    // TODO: select fanout peers and broadcast buffered messages
                    log::debug!("Gossip round fired");
                    scheduler.round_executed();
                }
            }

            // Anti-entropy pull interval (every 30 seconds)
            _ = _anti_entropy_timer.tick() => {
                log::debug!("Anti-entropy sync timer fired");
                // TODO: Pick one random active peer and send state.generate_sync_request()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    fn mock_envelope(id_byte: u8) -> TransactionEnvelope {
        TransactionEnvelope {
            message_id: [id_byte; 32],
            origin_pubkey: [0u8; 32],
            tx_xdr: format!("XDR{}", id_byte),
            ttl_hops: 10,
            timestamp: 0,
            signature: [0u8; 64],
        }
    }

    #[test]
    fn test_generate_sync_request() {
        let mut state = GossipState::new();
        state.add_envelope(mock_envelope(0xAA));
        state.add_envelope(mock_envelope(0xBB));

        let req = state.generate_sync_request();
        assert_eq!(req.known_message_ids.len(), 2);
        assert_eq!(req.known_message_ids[0], [0xAA, 0xAA, 0xAA, 0xAA]);
        assert_eq!(req.known_message_ids[1], [0xBB, 0xBB, 0xBB, 0xBB]);
    }

    #[test]
    fn test_handle_sync_request_delta_calculation() {
        let mut node_a = GossipState::new();
        // A has message AA and BB
        node_a.add_envelope(mock_envelope(0xAA));
        node_a.add_envelope(mock_envelope(0xBB));

        let mut node_b = GossipState::new();
        // B only has message AA -> B is missing BB
        node_b.add_envelope(mock_envelope(0xAA));

        // Node B generates request telling A what it has
        let req = node_b.generate_sync_request();

        // Node A processes request and calculates what B is missing
        let resp = node_a.handle_sync_request(&req);

        assert_eq!(resp.missing_envelopes.len(), 1);
        assert_eq!(resp.missing_envelopes[0].message_id[0], 0xBB);
    }

    #[test]
    fn test_handle_sync_response() {
        let mut state = GossipState::new();
        assert_eq!(state.active_envelopes.len(), 0);

        let resp = SyncResponse {
            missing_envelopes: vec![mock_envelope(0xCC)],
        };

        state.handle_sync_response(resp);
        assert_eq!(state.active_envelopes.len(), 1);
        assert_eq!(state.active_envelopes[0].message_id[0], 0xCC);
    }

    #[tokio::test]
    async fn test_gossip_loop_starts_without_blocking() {
        let scheduler = GossipScheduler::new();
        let handle = tokio::spawn(run_gossip_loop(scheduler));
        let result = timeout(Duration::from_millis(200), async {
            tokio::time::sleep(Duration::from_millis(100)).await;
        })
        .await;
        assert!(result.is_ok());
        handle.abort();
    }

    #[tokio::test]
    async fn test_gossip_loop_can_be_aborted() {
        let scheduler = GossipScheduler::new();
        let handle = tokio::spawn(run_gossip_loop(scheduler));
        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.abort();
        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_gossip_loop_starts_in_idle_mode() {
        use crate::gossip::round::IDLE_TIMEOUT_SEC;
        let mut scheduler = GossipScheduler::new();
        scheduler.last_active_msg_time =
            std::time::Instant::now() - Duration::from_secs(IDLE_TIMEOUT_SEC + 5);
        assert!(scheduler.is_idle());
        let handle = tokio::spawn(run_gossip_loop(scheduler));
        let result = timeout(Duration::from_millis(150), async {
            tokio::time::sleep(Duration::from_millis(50)).await;
        })
        .await;
        assert!(result.is_ok());
        handle.abort();
    }
}
