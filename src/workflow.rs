use crate::{
    adapters::conditions::Condition,
    ipfs::IPFSClient,
    lit::encrypt_with_public_key,
    nibble::{Adapter, Nibble},
    utils::generate_unique_id,
};
use ethers::{
    abi::{Abi, Token, Tokenize},
    middleware::SignerMiddleware,
    prelude::*,
    types::H160,
    utils::hex,
};
use serde::Serialize;
use serde_json::{Map, Value};

use std::{error::Error, fs::File, io::Read, path::Path, sync::Arc};

#[derive(Debug, Clone)]
pub struct Workflow {
    pub id: Vec<u8>,
    pub name: String,
    pub nodes: Vec<WorkflowNode>,
    pub links: Vec<WorkflowLink>,
    pub nibble_context: Arc<Nibble>,
    pub encrypted: bool,
}

#[derive(Debug, Clone)]
pub struct WorkflowNode {
    pub id: Vec<u8>,
    pub name: String,
    pub adapter_type: Adapter,
    pub adapter_id: Vec<u8>,
    pub metadata: Option<String>,
}

impl WorkflowNode {
    pub fn to_json(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("id".to_string(), Value::String(hex::encode(&self.id)));
        map.insert("name".to_string(), Value::String(self.name.clone()));
        map.insert(
            "adapter_type".to_string(),
            Value::String(format!("{:?}", self.adapter_type)),
        );
        map.insert(
            "adapter_id".to_string(),
            Value::String(hex::encode(&self.adapter_id)),
        );
        if let Some(metadata) = &self.metadata {
            map.insert("metadata".to_string(), Value::String(metadata.clone()));
        }
        map
    }
}

#[derive(Debug, Clone)]
pub struct WorkflowLink {
    pub from_node: Vec<u8>,
    pub to_node: Vec<u8>,
    pub condition: Option<Condition>,
}

impl WorkflowLink {
    pub fn to_json(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert(
            "from_node".to_string(),
            Value::String(hex::encode(&self.from_node)),
        );
        map.insert(
            "to_node".to_string(),
            Value::String(hex::encode(&self.to_node)),
        );
        if let Some(condition) = &self.condition {
            map.insert("condition".to_string(), Value::Object(condition.to_json()));
        }
        map
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ModifyWorkflow {
    pub id: Vec<u8>,
    pub metadata: String,
    pub encrypted: bool,
}

impl Tokenize for ModifyWorkflow {
    fn into_tokens(self) -> Vec<Token> {
        vec![
            Token::Bytes(self.id),
            Token::String(self.metadata),
            Token::Bool(self.encrypted),
        ]
    }
}

impl Workflow {
    pub fn add_node(
        &mut self,
        name: &str,
        adapter_type: Adapter,
        adapter_id: Vec<u8>,
        metadata: Option<String>,
        address: &H160,
    ) -> Result<(), String> {
        self.nodes.push(WorkflowNode {
            id: generate_unique_id(address),
            name: name.to_string(),
            adapter_type,
            adapter_id,
            metadata,
        });
        Ok(())
    }

    pub fn add_link(
        &mut self,
        from_node: Vec<u8>,
        to_node: Vec<u8>,
        condition: Option<Condition>,
    ) -> Result<(), String> {
        self.links.push(WorkflowLink {
            from_node,
            to_node,
            condition,
        });
        Ok(())
    }

    pub async fn remove(&mut self) -> Result<(), Box<dyn Error>> {
        if self.nibble_context.contracts.len() < 1 {
            return Err("No contracts found. Load or create a Nibble.".into());
        }

        let client = SignerMiddleware::new(
            self.nibble_context.provider.clone(),
            self.nibble_context
                .owner_wallet
                .clone()
                .with_chain_id(self.nibble_context.chain),
        );
        let client = Arc::new(client);

        let storage_contract_address = self
            .nibble_context
            .contracts
            .iter()
            .find(|c| c.name == "NibbleStorage")
            .ok_or("NibbleStorage contract not found")?
            .address;

        let mut abi_file = File::open(Path::new("./abis/NibbleStorage.json"))?;
        let mut abi_content = String::new();
        abi_file.read_to_string(&mut abi_content)?;
        let abi = serde_json::from_str::<Abi>(&abi_content)?;
        let contract_instance = Contract::new(storage_contract_address, abi, client.clone());

        let method = contract_instance.method::<_, H256>("removeWorkflow", self.id.clone());

        match method {
            Ok(call) => {
                let FunctionCall { tx, .. } = call;

                if let Some(tx_request) = tx.as_eip1559_ref() {
                    let gas_price = U256::from(500_000_000_000u64);
                    let max_priority_fee = U256::from(25_000_000_000u64);
                    let gas_limit = U256::from(300_000);

                    let cliente = contract_instance.client().clone();
                    let req = Eip1559TransactionRequest {
                        from: Some(client.address()),
                        to: Some(NameOrAddress::Address(storage_contract_address)),
                        gas: Some(gas_limit),
                        value: tx_request.value,
                        data: tx_request.data.clone(),
                        max_priority_fee_per_gas: Some(max_priority_fee),
                        max_fee_per_gas: Some(gas_price + max_priority_fee),
                        chain_id: Some(Chain::PolygonAmoy.into()),
                        ..Default::default()
                    };

                    let pending_tx = cliente.send_transaction(req, None).await.map_err(|e| {
                        eprintln!("Error sending the transaction: {:?}", e);
                        Box::<dyn std::error::Error>::from(format!(
                            "Error sending the transaction: {}",
                            e
                        ))
                    })?;

                    match pending_tx.await {
                        Ok(Some(receipt)) => receipt,
                        Ok(None) => {
                            return Err("Transaction not recieved".into());
                        }
                        Err(e) => {
                            eprintln!("Error with the transaction: {:?}", e);
                            return Err(e.into());
                        }
                    };
                } else {
                    return Err("EIP-1559 reference invalid.".into());
                }
            }
            Err(e) => {
                eprintln!(
                    "Error while preparing the method of addOrModifyAdaptersBatch: {}",
                    e
                );
                return Err(e.into());
            }
        }

        self.nodes.clear();
        self.links.clear();
        self.id = generate_unique_id(&self.nibble_context.owner_wallet.address());

        Ok(())
    }

    pub async fn persist(&self) -> Result<(), Box<dyn Error>> {
        let client = SignerMiddleware::new(
            self.nibble_context.provider.clone(),
            self.nibble_context
                .owner_wallet
                .clone()
                .with_chain_id(self.nibble_context.chain),
        );
        let client = Arc::new(client);

        let storage_contract_address = self
            .nibble_context
            .contracts
            .iter()
            .find(|c| c.name == "NibbleStorage")
            .ok_or("NibbleStorage contract not found")?
            .address;

        let mut abi_file = File::open(Path::new("./abis/NibbleStorage.json"))?;
        let mut abi_content = String::new();
        abi_file.read_to_string(&mut abi_content)?;
        let abi = serde_json::from_str::<Abi>(&abi_content)?;
        let contract_instance = Contract::new(storage_contract_address, abi, client.clone());

        let workflow = self
            .build_workflow(self.nibble_context.ipfs_client.as_ref())
            .await?;

        let method = contract_instance.method::<_, H256>("addOrModifyWorkflow", workflow);

        match method {
            Ok(call) => {
                let FunctionCall { tx, .. } = call;

                if let Some(tx_request) = tx.as_eip1559_ref() {
                    let gas_price = U256::from(500_000_000_000u64);
                    let max_priority_fee = U256::from(25_000_000_000u64);
                    let gas_limit = U256::from(300_000);

                    let cliente = contract_instance.client().clone();
                    let req = Eip1559TransactionRequest {
                        from: Some(client.address()),
                        to: Some(NameOrAddress::Address(storage_contract_address)),
                        gas: Some(gas_limit),
                        value: tx_request.value,
                        data: tx_request.data.clone(),
                        max_priority_fee_per_gas: Some(max_priority_fee),
                        max_fee_per_gas: Some(gas_price + max_priority_fee),
                        chain_id: Some(Chain::PolygonAmoy.into()),
                        ..Default::default()
                    };

                    let pending_tx = cliente.send_transaction(req, None).await.map_err(|e| {
                        eprintln!("Error sending the transaction: {:?}", e);
                        Box::<dyn std::error::Error>::from(format!(
                            "Error sending the transaction: {}",
                            e
                        ))
                    })?;

                    match pending_tx.await {
                        Ok(Some(receipt)) => receipt,
                        Ok(None) => {
                            return Err("Transaction not recieved".into());
                        }
                        Err(e) => {
                            eprintln!("Error with the transaction: {:?}", e);
                            return Err(e.into());
                        }
                    };
                } else {
                    return Err("EIP-1559 reference invalid.".into());
                }
            }
            Err(e) => {
                eprintln!(
                    "Error while preparing the method of addOrModifyAdaptersBatch: {}",
                    e
                );
                return Err(e.into());
            }
        }

        Ok(())
    }

    pub async fn execute(&self) -> Result<(), Box<dyn Error>> {
        for node in &self.nodes {
            if let Some(link) = self.links.iter().find(|conn| conn.to_node == node.id) {
                if let Some(condition) = &link.condition {
                    if !(condition.check.condition_fn)(serde_json::json!(true)) {
                        continue;
                    }
                }
            }

            match node.adapter_type {
                Adapter::Agent => {
                    let agent = self
                        .nibble_context
                        .agents
                        .iter()
                        .find(|a| a.id == node.adapter_id)
                        .ok_or("Agent not found")?;
                    println!("Executing Agent: {:?}", agent);
                }
                Adapter::Condition => {}
                Adapter::Listener => {}
                Adapter::FHEGate => {}
                Adapter::Evaluation => {}
                Adapter::OnChainConnector | Adapter::OffChainConnector => {}
            }
        }
        Ok(())
    }

    async fn build_workflow(
        &self,
        ipfs_client: &dyn IPFSClient,
    ) -> Result<ModifyWorkflow, Box<dyn Error>> {
        let mut metadata_map = Map::new();
        metadata_map.insert(
            "links".to_string(),
            Value::Array(
                self.links
                    .iter()
                    .map(|link| Value::Object(link.to_json()))
                    .collect(),
            ),
        );
        metadata_map.insert(
            "nodes".to_string(),
            Value::Array(
                self.nodes
                    .iter()
                    .map(|node| Value::Object(node.to_json()))
                    .collect(),
            ),
        );

        let mut metadata = serde_json::to_vec(&metadata_map)?;

        if self.encrypted {
            metadata = encrypt_with_public_key(metadata, self.nibble_context.owner_wallet.clone())?;
        }
        let ipfs_hash = ipfs_client.upload(metadata).await?;

        Ok(ModifyWorkflow {
            id: self.id.clone(),
            encrypted: self.encrypted,
            metadata: ipfs_hash,
        })
    }
}
