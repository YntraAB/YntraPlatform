//! Release Signature & Update Manifest Generator
//! Run with: cargo run --file packaging/updater/generate-release-signature.rs -- <PRIVATE_KEY_HEX> <BINARY_PATH> <VERSION> <DOWNLOAD_URL>

use std::env;
use std::fs;
use sha2::{Digest, Sha256};
use ed25519_dalek::{Signer, SigningKey};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 {
        println!("Usage: generate-release-signature <PRIVATE_KEY_HEX> <BINARY_PATH> <VERSION> <DOWNLOAD_URL>");
        std::process::exit(1);
    }

    let private_key_hex = &args[1];
    let binary_path = &args[2];
    let version = &args[3];
    let download_url = &args[4];

    // Read binary & compute SHA-256
    let binary_bytes = fs::read(binary_path).expect("Failed to read binary file");
    let mut hasher = Sha256::new();
    hasher.update(&binary_bytes);
    let sha256_hex = const_hex::encode(hasher.finalize());

    // Decode Ed25519 private key
    let priv_bytes = const_hex::decode(private_key_hex).expect("Invalid hex private key");
    let priv_arr: [u8; 32] = priv_bytes.try_into().expect("Private key must be 32 bytes");
    let signing_key = SigningKey::from_bytes(&priv_arr);

    // Construct signature message
    let mut message = Vec::new();
    message.extend_from_slice(b"YNTRA_BINARY_UPDATE_V1\0");
    message.extend_from_slice(&(version.len() as u64).to_be_bytes());
    message.extend_from_slice(version.as_bytes());
    message.extend_from_slice(&(download_url.len() as u64).to_be_bytes());
    message.extend_from_slice(download_url.as_bytes());
    message.extend_from_slice(&(sha256_hex.len() as u64).to_be_bytes());
    message.extend_from_slice(sha256_hex.as_bytes());

    // Sign message
    let signature = signing_key.sign(&message);
    let signature_hex = const_hex::encode(signature.to_bytes());
    let pub_key_hex = const_hex::encode(signing_key.verifying_key().to_bytes());

    println!("===================================================");
    println!(" Yntra Platform - Update Release Signature");
    println!("===================================================");
    println!("Version      : {}", version);
    println!("Public Key   : {}", pub_key_hex);
    println!("SHA-256      : {}", sha256_hex);
    println!("Signature    : {}", signature_hex);
    println!("Download URL : {}", download_url);
    println!("===================================================");
}
