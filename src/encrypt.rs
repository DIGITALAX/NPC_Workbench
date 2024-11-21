use ecies::{decrypt, encrypt};
use ethers::signers::LocalWallet;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{error::Error, io};

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
    let signer = wallet.signer();
    let encoded_point = signer.verifying_key().to_encoded_point(true);
    let public_key_bytes = encoded_point.as_bytes();

    if metadata.is_empty() {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid data.",
        )));
    }

    let encrypted_data = encrypt(public_key_bytes, metadata.as_slice()).map_err(|e| {
        Box::new(io::Error::new(
            io::ErrorKind::Other,
            format!("Error encrypting the data: {:?}", e),
        ))
    })?;

    Ok(encrypted_data)
}

pub fn decrypt_with_private_key(
    encrypted_data: Vec<u8>,
    wallet: LocalWallet,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let private_key_bytes = wallet.signer().to_bytes();

    if encrypted_data.is_empty() {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid data.",
        )));
    }

    let decrypted_data = decrypt(&private_key_bytes, encrypted_data.as_slice()).map_err(|e| {
        Box::new(io::Error::new(
            io::ErrorKind::Other,
            format!("Error decrypting the data: {:?}", e),
        ))
    })?;

    let json_value: Value = serde_json::from_slice(&decrypted_data).map_err(|e| {
        Box::new(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Error converting the data to JSON: {:?}", e),
        ))
    })?;

    Ok(json_value)
}
