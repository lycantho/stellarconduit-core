#[derive(thiserror::Error, Debug)]
pub enum SignError {
    #[error("Invalid Ed25519 signature")]
    InvalidSignature,

    #[error("Malformed public key: {0}")]
    MalformedPublicKey(String),
}
