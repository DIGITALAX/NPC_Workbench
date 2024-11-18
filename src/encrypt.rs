use ethers::signers::LocalWallet;
use k256::{
    elliptic_curve::{sec1::ToEncodedPoint, Field, PrimeField},
    ProjectivePoint, PublicKey, Scalar,
};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::error::Error;

#[derive(Serialize, Deserialize)]
struct EncryptParams {
    access_control_conditions: Vec<String>,
    evm_contract_conditions: Vec<String>,
    sol_rpc_conditions: Vec<String>,
    unified_access_control_conditions: Vec<String>,
    chain: String,
    data_to_encrypt: String,
    public_key: Vec<u8>,
    identity: Vec<u8>,
}

pub fn encrypt_with_public_key(
    metadata: Vec<u8>,
    wallet: LocalWallet,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let public_key = wallet.signer().verifying_key().to_encoded_point(false);

    let ephemeral_private_key = k256::Scalar::random(&mut OsRng);
    let ephemeral_public_key =
        (ProjectivePoint::GENERATOR * ephemeral_private_key).to_encoded_point(false);

    let recipient_public_key = PublicKey::from_sec1_bytes(public_key.as_bytes())?;

    let shared_secret_point = recipient_public_key.to_projective() * ephemeral_private_key;
    let shared_secret = shared_secret_point
        .to_encoded_point(false)
        .as_bytes()
        .to_vec();

    let encryption_key = Sha256::digest(shared_secret);

    let encrypted_message: Vec<u8> = metadata
        .iter()
        .zip(encryption_key.iter().cycle())
        .map(|(m, k)| m ^ k)
        .collect();

    let mut result = ephemeral_public_key.as_bytes().to_vec();
    result.extend_from_slice(&encrypted_message);

    Ok(result)
}

pub fn decrypt_with_private_key(
    encrypted_data: Vec<u8>,
    wallet: LocalWallet,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let private_key_bytes = wallet.signer().to_bytes();

    let ephemeral_public_key_bytes = &encrypted_data[..65];
    let encrypted_message = &encrypted_data[65..];

    let ephemeral_public_key =
        PublicKey::from_sec1_bytes(ephemeral_public_key_bytes)?.to_projective();

    let mut private_key_array = [0u8; 32];
    private_key_array.copy_from_slice(&private_key_bytes[..32]);
    let private_key_scalar = Scalar::from_repr(private_key_array.into())
        .unwrap_or_else(|| panic!("Invalid private key bytes."));
    let shared_secret_point = ephemeral_public_key * private_key_scalar;

    let shared_secret_bytes = shared_secret_point
        .to_encoded_point(false)
        .as_bytes()
        .to_vec();

    let decryption_key = Sha256::digest(shared_secret_bytes);

    let decrypted_message: Value = encrypted_message
        .iter()
        .zip(decryption_key.iter().cycle())
        .map(|(c, k)| c ^ k)
        .collect();

    Ok(decrypted_message)
}
