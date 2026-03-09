use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use std::fmt;

#[derive(Clone, Debug)]
pub struct PeerIdentity {
    /// Ed25519 public key bytes
    pub pubkey: [u8; 32],
    /// Hex-encoded string representation for logging
    pub display_id: String,
}

impl PeerIdentity {
    pub fn new(pubkey: [u8; 32]) -> Self {
        let display_id = pubkey.iter().map(|b| format!("{:02x}", b)).collect();
        Self { pubkey, display_id }
    }

    /// Verify an Ed25519 signature over `message` using this peer's public key.
    /// Returns `false` on bad key or failed verification.
    pub fn verify_signature(&self, message: &[u8], signature: &[u8; 64]) -> bool {
        let Ok(verifying_key) = VerifyingKey::from_bytes(&self.pubkey) else {
            return false;
        };
        let sig = Signature::from_bytes(signature);
        verifying_key.verify(message, &sig).is_ok()
    }
}

impl fmt::Display for PeerIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.display_id[..16])
    }
}
