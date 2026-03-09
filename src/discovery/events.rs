use crate::peer::identity::PeerIdentity;

#[derive(Clone)]
pub enum DiscoveryEvent {
    /// A new peer was seen for the first time
    PeerDiscovered(PeerIdentity),
    /// A known peer sent a fresh beacon; includes current signal strength (RSSI)
    PeerUpdated(PeerIdentity, u8),
    /// A peer has exceeded the expiry window and is considered offline
    PeerLost(PeerIdentity),
}
