//! WireGuard Curve25519 key pair generation

use base64::Engine;
use rand::rngs::OsRng;
use x25519_dalek::{PublicKey, StaticSecret};

/// Generated WireGuard key pair (Base64 encoded)
#[derive(Debug, Clone, serde::Serialize)]
pub struct WgKeyPair {
    pub private_key: String,
    pub public_key: String,
}

/// Generate a new Curve25519 key pair for WireGuard
pub fn generate_keypair() -> WgKeyPair {
    let secret = StaticSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&secret);

    WgKeyPair {
        private_key: base64::engine::general_purpose::STANDARD.encode(secret.as_bytes()),
        public_key: base64::engine::general_purpose::STANDARD.encode(public.as_bytes()),
    }
}
