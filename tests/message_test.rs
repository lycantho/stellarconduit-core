use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use stellarconduit_core::message::{
    signing::{sign_envelope, verify_signature},
    types::TransactionEnvelope,
};

fn make_test_envelope(keypair: &SigningKey, tx_xdr: &str) -> TransactionEnvelope {
    TransactionEnvelope {
        message_id: [0u8; 32],
        origin_pubkey: keypair.verifying_key().to_bytes(),
        tx_xdr: tx_xdr.to_string(),
        ttl_hops: 10,
        timestamp: 1672531200,
        signature: [0u8; 64],
    }
}

#[test]
fn test_sign_and_verify_success() {
    let mut csprng = OsRng;
    let keypair = SigningKey::generate(&mut csprng);
    let mut envelope = make_test_envelope(&keypair, "AAAAAQAAAAAAAAAA");

    sign_envelope(&keypair, &mut envelope).expect("signing should succeed");
    let result = verify_signature(&envelope).expect("verification should succeed");
    assert!(result, "signature should be valid");
}

#[test]
fn test_verify_tampered_tx_xdr() {
    let mut csprng = OsRng;
    let keypair = SigningKey::generate(&mut csprng);
    let mut envelope = make_test_envelope(&keypair, "AAAAAQAAAAAAAAAA");

    sign_envelope(&keypair, &mut envelope).expect("signing should succeed");

    // Tamper with the payload after signing
    envelope.tx_xdr = "TAMPERED_PAYLOAD_XDR".to_string();

    let result = verify_signature(&envelope);
    assert!(
        result.is_err(),
        "verification should fail due to tampered tx_xdr"
    );
}

#[test]
fn test_verify_tampered_timestamp() {
    let mut csprng = OsRng;
    let keypair = SigningKey::generate(&mut csprng);
    let mut envelope = make_test_envelope(&keypair, "AAAAAQAAAAAAAAAA");

    sign_envelope(&keypair, &mut envelope).expect("signing should succeed");

    // Tamper with the timestamp after signing
    envelope.timestamp += 1;

    let result = verify_signature(&envelope);
    assert!(
        result.is_err(),
        "verification should fail due to tampered timestamp"
    );
}

#[test]
fn test_verify_wrong_key() {
    let mut csprng = OsRng;
    let keypair_a = SigningKey::generate(&mut csprng);
    let keypair_b = SigningKey::generate(&mut csprng);

    let mut envelope = make_test_envelope(&keypair_a, "AAAAAQAAAAAAAAAA");
    // Sign with key A but put key B's pubkey as origin
    sign_envelope(&keypair_a, &mut envelope).expect("signing should succeed");
    // Swap the pubkey to a different one
    envelope.origin_pubkey = keypair_b.verifying_key().to_bytes();

    let result = verify_signature(&envelope);
    assert!(
        result.is_err(),
        "verification should fail when origin_pubkey doesn't match signing key"
    );
}

#[test]
fn test_verify_invalid_signature() {
    let mut csprng = OsRng;
    let keypair = SigningKey::generate(&mut csprng);
    let mut envelope = make_test_envelope(&keypair, "AAAAAQAAAAAAAAAA");

    sign_envelope(&keypair, &mut envelope).expect("signing should succeed");

    // Corrupt the signature bytes
    envelope.signature[0] ^= 0xFF;
    envelope.signature[1] ^= 0xFF;

    let result = verify_signature(&envelope);
    assert!(
        result.is_err(),
        "verification should fail with corrupted signature bytes"
    );
}
