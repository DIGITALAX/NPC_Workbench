use crate::{nibble::Adaptable, utils::generate_unique_id};
use bincode::{deserialize, serialize};
use ethers::{
    abi::Abi,
    contract::Contract,
    middleware::SignerMiddleware,
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    types::{Chain, H160},
};
use serde::{Deserialize, Serialize};
use std::{error::Error, fs::File, io::Read, path::Path, sync::Arc};
use tfhe::{generate_keys, prelude::*, ClientKey, ConfigBuilder, FheUint8, ServerKey};

#[derive(Debug, Clone)]
pub struct FHEGate {
    pub name: String,
    pub id: String,
    pub key: String,
    pub encrypted: bool,
    pub contract_address: H160,
    pub operation: String,
    pub chain: Chain,
}

pub fn configure_new_gate(
    name: &str,
    key: &str,
    encrypted: bool,
    address: &H160,
    contract_address: &H160,
    operation: &str,
    chain: Chain,
) -> Result<FHEGate, Box<dyn Error + Send + Sync>> {
    let fhe_gate = FHEGate {
        name: name.to_string(),
        id: generate_unique_id(address),
        key: key.to_string(),
        encrypted,
        contract_address: *contract_address,
        operation: operation.to_string(),
        chain,
    };
    Ok(fhe_gate)
}

impl Adaptable for FHEGate {
    fn name(&self) -> &str {
        &self.name
    }
    fn id(&self) -> &str {
        &self.id
    }
}

impl FHEGate {
    pub async fn check_fhe_gate(
        &self,
        encrypted_value: Vec<u8>,
        criterion: Option<Vec<u8>>,
        client_key: ClientKey,
        provider: Provider<Http>,
        wallet: LocalWallet,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let client = SignerMiddleware::new(provider, wallet.with_chain_id(self.chain));
        let client = Arc::new(client);

        let mut abi_file = File::open(Path::new("./../../../abis/FHEGate.json"))?;
        let mut abi_content = String::new();
        abi_file.read_to_string(&mut abi_content)?;
        let abi = serde_json::from_str::<Abi>(&abi_content)?;

        let contract = Contract::new(self.contract_address, abi, client.clone());

        let operation_name = self.operation.clone();
        let result_encrypted: Vec<u8> = if let Some(criterion_value) = criterion {
            contract
                .method::<_, Vec<u8>>(&operation_name, (encrypted_value.clone(), criterion_value))
                .map_err(|e| format!("Error creating contract method: {}", e))?
                .call()
                .await
                .map_err(|e| format!("Error calling contract method '{}': {}", operation_name, e))?
        } else {
            contract
                .method::<_, Vec<u8>>(&operation_name, (encrypted_value.clone(),))
                .map_err(|e| format!("Error creating contract method without criterion: {}", e))?
                .call()
                .await
                .map_err(|e| format!("Error calling contract method '{}': {}", operation_name, e))?
        };
        println!(
            "Encrypted result from operation '{}': {:?}",
            operation_name, result_encrypted
        );

        let result: bool = contract
            .method::<_, bool>("isValid", result_encrypted.clone())
            .map_err(|e| format!("Error creating isValid method: {}", e))?
            .call()
            .await
            .map_err(|e| format!("Error calling isValid: {}", e))?;

        if result {
            println!("FHE Gate validation passed.");
            Ok(true)
        } else {
            println!("FHE Gate validation failed.");
            Ok(false)
        }
    }
}

pub fn decrypt_fhe<T>(
    client_key: ClientKey,
    encrypted_data: Vec<FheUint8>,
) -> Result<T, Box<dyn Error>>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    let mut decrypted_data = Vec::new();
    for encrypted_byte in encrypted_data.iter() {
        let decrypted_byte: u8 = encrypted_byte.decrypt(&client_key);
        decrypted_data.push(decrypted_byte);
    }

    let deserialized_data: T = deserialize(&decrypted_data)?;

    Ok(deserialized_data)
}

pub fn encrypt_fhe<T>(data: &T) -> Result<(ClientKey, ServerKey, Vec<FheUint8>), Box<dyn Error>>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    let config = ConfigBuilder::default().build();

    let (client_key, server_keys) = generate_keys(config);

    let serialized_data = serialize(data)?;

    let mut encrypted_data = Vec::new();
    for byte in serialized_data.iter() {
        let encrypted_byte = FheUint8::try_encrypt(*byte, &client_key)?;
        encrypted_data.push(encrypted_byte);
    }

    Ok((client_key, server_keys, encrypted_data))
}
