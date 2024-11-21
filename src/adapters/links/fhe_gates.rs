use ethers::{
    abi::Abi,
    contract::Contract,
    middleware::SignerMiddleware,
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    types::{Chain, H160},
};
use std::{error::Error, fs::File, io::Read, path::Path, sync::Arc};

use crate::{nibble::Adaptable, utils::generate_unique_id};

#[derive(Debug, Clone)]
pub struct FHEGate {
    pub name: String,
    pub id: Vec<u8>,
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
    fn id(&self) -> &Vec<u8> {
        &self.id
    }
}

impl FHEGate {
    pub async fn check_fhe_gate(
        &self,
        encrypted_value: Vec<u8>,
        criterion: Option<Vec<u8>>,
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
