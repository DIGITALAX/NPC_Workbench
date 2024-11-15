use crate::{
    adapters::{
        agents::Agent,
        conditions::Condition,
        connectors::{off_chain::OffChainConnector, on_chain::OnChainConnector},
        evaluations::Evaluation,
        fhe_gates::FHEGate,
        listeners::Listener,
    },
    ipfs::{IPFSClient, IPFSClientFactory, IPFSProvider},
    nibble::ContractInfo,
};
use chrono::Utc;
use dotenv::dotenv;
use ethers::{abi::Token, types::U256};
use rand::Rng;
use serde_json::Value;
use std::{collections::HashMap, env, error::Error, sync::Arc};

pub struct GraphNibbleResponse {
    pub agents: Vec<Agent>,
    pub conditions: Vec<Condition>,
    pub listeners: Vec<Listener>,
    pub fhe_gates: Vec<FHEGate>,
    pub evaluations: Vec<Evaluation>,
    pub onchain_connectors: Vec<OnChainConnector>,
    pub offchain_connectors: Vec<OffChainConnector>,
    pub contracts: Vec<ContractInfo>,
    pub count: U256,
}

pub fn convert_value_to_token(value: &Value) -> Result<Token, Box<dyn Error>> {
    match value {
        Value::Number(num) if num.is_u64() => Ok(Token::Uint(U256::from(num.as_u64().unwrap()))),
        Value::String(s) => Ok(Token::String(s.clone())),
        _ => Err("Unsupported parameter type".into()),
    }
}

pub fn load_ipfs_client() -> Result<Arc<dyn IPFSClient>, Box<dyn Error>> {
    dotenv().ok();
    let provider = match env::var("IPFS_PROVIDER")?.as_str() {
        "Infura" => IPFSProvider::Infura,
        "Pinata" => IPFSProvider::Pinata,
        "Custom" => IPFSProvider::Custom,
        _ => return Err("Unsupported IPFS provider".into()),
    };

    let mut config = HashMap::new();
    match provider {
        IPFSProvider::Infura => {
            config.insert("project_id".to_string(), env::var("INFURA_PROJECT_ID")?);
            config.insert(
                "project_secret".to_string(),
                env::var("INFURA_PROJECT_SECRET")?,
            );
        }
        IPFSProvider::Pinata => {
            config.insert("api_key".to_string(), env::var("PINATA_API_KEY")?);
            config.insert(
                "secret_api_key".to_string(),
                env::var("PINATA_SECRET_API_KEY")?,
            );
        }
        IPFSProvider::Custom => {
            config.insert("api_url".to_string(), env::var("IPFS_API_URL")?);

            for (key, value) in env::vars() {
                if key.starts_with("IPFS_CUSTOM_HEADER_") {
                    let header_key = key.trim_start_matches("IPFS_CUSTOM_HEADER_").to_string();
                    config.insert(header_key, value);
                }
            }
        }
    }

    IPFSClientFactory::create_client(provider, config)
}

pub fn generate_unique_id() -> Vec<u8> {
    let timestamp = Utc::now().timestamp_nanos_opt().expect("Invalid timestamp");
    let random_bytes: [u8; 4] = rand::thread_rng().gen();
    let mut unique_id = Vec::with_capacity(12);
    unique_id.extend_from_slice(&timestamp.to_be_bytes());
    unique_id.extend_from_slice(&random_bytes);
    unique_id
}

pub async fn load_nibble_from_subgraph(id: Vec<u8>) -> Result<GraphNibbleResponse, Box<dyn Error>> {
}
