//! BLE peer discovery module for StellarConduit.
//!
//! Provides `BleAdvertiser` (broadcasts this device's identity) and `BleScanner`
//! (passively listens for advertisements from nearby StellarConduit devices and
//! maintains the `PeerList`).
//!
//! Platform note: `btleplug` requires a hardware or virtual BLE adapter at runtime.
//! Unit tests in this module exercise pure Rust logic only.

use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::Mutex;

use crate::discovery::events::DiscoveryEvent;
use crate::discovery::peer_list::PeerList;
use crate::peer::identity::PeerIdentity;
use crate::transport::errors::TransportError;

// Re-export SC_SERVICE_UUID from the transport layer so the scanner and
// advertiser use the same UUID as the GATT server/client.
pub use crate::transport::ble_transport::{
    SC_NOTIFY_CHAR_UUID, SC_SERVICE_UUID, SC_WRITE_CHAR_UUID,
};

// ─── BleAdvertisementPayload ──────────────────────────────────────────────────

/// Capability flags embedded in the BLE advertisement manufacturer data.
/// bit 0 = node is a relay, bit 1 = node has Wi-Fi Direct support.
#[derive(Debug, Clone, PartialEq)]
pub struct BleAdvertisementPayload {
    /// 32-byte Ed25519 public key of the advertising peer.
    pub pubkey: [u8; 32],
    /// Capability bitmask: bit 0 = is_relay_node, bit 1 = has_wifi_direct.
    pub caps: u8,
}

impl BleAdvertisementPayload {
    /// Encode into exactly 33 bytes: pubkey (32) || caps (1).
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(33);
        buf.extend_from_slice(&self.pubkey);
        buf.push(self.caps);
        buf
    }

    /// Decode from a raw byte slice. Returns `None` if the slice is < 33 bytes.
    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < 33 {
            return None;
        }
        let mut pubkey = [0u8; 32];
        pubkey.copy_from_slice(&data[0..32]);
        let caps = data[32];
        Some(Self { pubkey, caps })
    }

    /// Build capability flags from semantic booleans.
    pub fn build_caps(is_relay: bool, has_wifi_direct: bool) -> u8 {
        let mut caps = 0u8;
        if is_relay {
            caps |= 0b0000_0001;
        }
        if has_wifi_direct {
            caps |= 0b0000_0010;
        }
        caps
    }

    /// Whether the advertising node is a relay.
    pub fn is_relay(&self) -> bool {
        self.caps & 0b0000_0001 != 0
    }

    /// Whether the advertising node supports Wi-Fi Direct.
    pub fn has_wifi_direct(&self) -> bool {
        self.caps & 0b0000_0010 != 0
    }
}

// ─── BleScanner ───────────────────────────────────────────────────────────────

/// Continuously scans for BLE advertisements from other StellarConduit devices.
///
/// When a device advertising `SC_SERVICE_UUID` is found, the scanner:
/// 1. Decodes the `BleAdvertisementPayload` from the manufacturer data.
/// 2. Calls `PeerList::insert_or_update` with the peer's pubkey and RSSI.
/// 3. Broadcasts the resulting `DiscoveryEvent` to all subscribers.
pub struct BleScanner {
    peer_list: Arc<Mutex<PeerList>>,
    event_tx: broadcast::Sender<DiscoveryEvent>,
}

impl BleScanner {
    /// Start the BLE scanner.
    ///
    /// In a real device context this method would acquire a `btleplug` manager and adapter,
    /// start a filtered scan for `SC_SERVICE_UUID`, and spawn an async task that reads
    /// `ScanFilter` events in a loop.
    pub async fn start(
        peer_list: Arc<Mutex<PeerList>>,
    ) -> Result<(Self, broadcast::Receiver<DiscoveryEvent>), TransportError> {
        let (tx, rx) = broadcast::channel(128);
        let scanner = Self {
            peer_list,
            event_tx: tx,
        };
        // Platform integration: btleplug adapter + scan start would happen here.
        Ok((scanner, rx))
    }

    /// Stop the BLE scanner.
    ///
    /// In a real device context, this would call `adapter.stop_scan()`.
    pub async fn stop(&mut self) {
        // Platform integration: adapter.stop_scan().await would happen here.
    }

    /// Process a single BLE advertisement event.
    ///
    /// This method encapsulates the core discovery logic and is directly unit-testable
    /// without a real BLE adapter. In production it is called from the btleplug scan loop.
    ///
    /// `manufacturer_data` — raw bytes from the advertisement's manufacturer data field.
    /// `rssi`              — received signal strength indicator (0–255 mapped from –dBm).
    pub async fn handle_advertisement(
        &self,
        manufacturer_data: &[u8],
        rssi: u8,
    ) -> Option<DiscoveryEvent> {
        let payload = BleAdvertisementPayload::decode(manufacturer_data)?;
        let mut list = self.peer_list.lock().await;
        let event = list.insert_or_update(payload.pubkey, rssi)?;

        // Best-effort broadcast — it's fine if there are no active receivers.
        let _ = self.event_tx.send(event.clone());

        Some(event)
    }
}

// ─── BleAdvertiser ────────────────────────────────────────────────────────────

/// Advertises this node's identity so nearby `BleScanner` instances can discover it.
///
/// Encodes the local `PeerIdentity` pubkey and capability flags into a
/// `BleAdvertisementPayload` and broadcasts it via a BLE advertisement containing
/// `SC_SERVICE_UUID`.
pub struct BleAdvertiser {
    identity: PeerIdentity,
    is_relay: bool,
    is_running: bool,
}

impl BleAdvertiser {
    /// Start BLE advertising.
    ///
    /// In a real device context this method would:
    /// 1. Acquire a `btleplug` manager and adapter.
    /// 2. Encode the `BleAdvertisementPayload` (pubkey + caps).
    /// 3. Build an advertisement packet with `SC_SERVICE_UUID` + manufacturer data.
    /// 4. Call `adapter.start_advertising()`.
    pub async fn start(identity: PeerIdentity, is_relay: bool) -> Result<Self, TransportError> {
        // Platform integration: btleplug adapter acquisition and advertising would happen here.
        Ok(Self {
            identity,
            is_relay,
            is_running: true,
        })
    }

    /// Stop BLE advertising.
    ///
    /// In a real device context this would call `adapter.stop_advertising()`.
    pub async fn stop(&mut self) {
        self.is_running = false;
        // Platform integration: adapter.stop_advertising().await.
    }

    /// Returns whether the advertiser is currently running.
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Build the payload that this advertiser embeds in BLE manufacturer data.
    pub fn build_payload(&self) -> BleAdvertisementPayload {
        BleAdvertisementPayload {
            pubkey: self.identity.pubkey,
            caps: BleAdvertisementPayload::build_caps(self.is_relay, false),
        }
    }
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn pk(b: u8) -> [u8; 32] {
        [b; 32]
    }

    // ── BleAdvertisementPayload encode / decode ────────────────────────────────

    #[test]
    fn encode_is_exactly_33_bytes() {
        let p = BleAdvertisementPayload {
            pubkey: pk(0xAA),
            caps: 0b11,
        };
        assert_eq!(p.encode().len(), 33);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let original = BleAdvertisementPayload {
            pubkey: pk(0x42),
            caps: 0b01,
        };
        let decoded = BleAdvertisementPayload::decode(&original.encode()).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_rejects_short_slice() {
        assert!(BleAdvertisementPayload::decode(&[0u8; 32]).is_none());
    }

    #[test]
    fn decode_accepts_extra_trailing_bytes() {
        let mut data = [0u8; 40];
        data[32] = 0b10;
        let p = BleAdvertisementPayload::decode(&data).unwrap();
        assert_eq!(p.caps, 0b10);
    }

    // ── Capability flag helpers ────────────────────────────────────────────────

    #[test]
    fn relay_flag_is_bit_0() {
        let caps = BleAdvertisementPayload::build_caps(true, false);
        assert_eq!(caps & 0b01, 1);
        assert_eq!(caps & 0b10, 0);
    }

    #[test]
    fn wifi_direct_flag_is_bit_1() {
        let caps = BleAdvertisementPayload::build_caps(false, true);
        assert_eq!(caps & 0b01, 0);
        assert_eq!(caps & 0b10, 2);
    }

    #[test]
    fn is_relay_and_has_wifi_direct_both_set() {
        let p = BleAdvertisementPayload {
            pubkey: pk(1),
            caps: BleAdvertisementPayload::build_caps(true, true),
        };
        assert!(p.is_relay());
        assert!(p.has_wifi_direct());
    }

    #[test]
    fn no_flags_set() {
        let p = BleAdvertisementPayload {
            pubkey: pk(2),
            caps: 0,
        };
        assert!(!p.is_relay());
        assert!(!p.has_wifi_direct());
    }

    // ── BleScanner::handle_advertisement ──────────────────────────────────────

    #[tokio::test]
    async fn scanner_fires_peer_discovered_on_first_contact() {
        let peer_list = Arc::new(Mutex::new(PeerList::new(300)));
        let (scanner, _rx) = BleScanner::start(peer_list).await.unwrap();

        let payload = BleAdvertisementPayload {
            pubkey: pk(0x55),
            caps: 0,
        };
        let event = scanner.handle_advertisement(&payload.encode(), 80).await;

        assert!(matches!(event, Some(DiscoveryEvent::PeerDiscovered(_))));
    }

    #[tokio::test]
    async fn scanner_fires_peer_updated_on_repeat_contact() {
        let peer_list = Arc::new(Mutex::new(PeerList::new(300)));
        let (scanner, _rx) = BleScanner::start(peer_list).await.unwrap();

        let payload = BleAdvertisementPayload {
            pubkey: pk(0x66),
            caps: 0,
        };
        let encoded = payload.encode();

        // First contact
        scanner.handle_advertisement(&encoded, 70).await;
        // Second contact — same peer
        let event = scanner.handle_advertisement(&encoded, 75).await;

        assert!(matches!(event, Some(DiscoveryEvent::PeerUpdated(_, _))));
    }

    #[tokio::test]
    async fn scanner_returns_none_for_malformed_payload() {
        let peer_list = Arc::new(Mutex::new(PeerList::new(300)));
        let (scanner, _rx) = BleScanner::start(peer_list).await.unwrap();

        // Only 10 bytes — too short to decode
        let result = scanner.handle_advertisement(&[0u8; 10], 50).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn scanner_broadcasts_event_to_receiver() {
        let peer_list = Arc::new(Mutex::new(PeerList::new(300)));
        let (scanner, mut rx) = BleScanner::start(peer_list).await.unwrap();

        let payload = BleAdvertisementPayload {
            pubkey: pk(0x77),
            caps: 0,
        };
        scanner.handle_advertisement(&payload.encode(), 90).await;

        let received = rx.try_recv();
        assert!(received.is_ok(), "Expected a broadcast event");
    }

    // ── BleAdvertiser ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn advertiser_starts_in_running_state() {
        let identity = PeerIdentity::new([0xBBu8; 32]);
        let adv = BleAdvertiser::start(identity, false).await.unwrap();
        assert!(adv.is_running());
    }

    #[tokio::test]
    async fn advertiser_stop_clears_running_flag() {
        let identity = PeerIdentity::new([0xCCu8; 32]);
        let mut adv = BleAdvertiser::start(identity, false).await.unwrap();
        adv.stop().await;
        assert!(!adv.is_running());
    }

    #[tokio::test]
    async fn advertiser_build_payload_encodes_relay_flag() {
        let identity = PeerIdentity::new([0xDDu8; 32]);
        let adv = BleAdvertiser::start(identity, true).await.unwrap();
        let payload = adv.build_payload();
        assert!(payload.is_relay());
        assert_eq!(payload.pubkey, [0xDDu8; 32]);
    }

    #[tokio::test]
    async fn advertiser_payload_encodes_to_33_bytes() {
        let identity = PeerIdentity::new([0xEEu8; 32]);
        let adv = BleAdvertiser::start(identity, false).await.unwrap();
        assert_eq!(adv.build_payload().encode().len(), 33);
    }
}
