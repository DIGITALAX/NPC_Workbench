use crate::ipfs::{IPFSClient, IPFSClientFactory, IPFSProvider};
use dotenv::dotenv;
use ethers::types::U256;
use serde_json::Value;
use std::{collections::HashMap, env, error::Error};

pub fn convert_value_to_token(value: &Value) -> Result<ethers::abi::Token, Box<dyn Error>> {
    match value {
        Value::Number(num) if num.is_u64() => {
            Ok(ethers::abi::Token::Uint(U256::from(num.as_u64().unwrap())))
        }
        Value::String(s) => Ok(ethers::abi::Token::String(s.clone())),
        _ => Err("Unsupported parameter type".into()),
    }
}

pub fn load_ipfs_client() -> Result<Box<dyn IPFSClient>, Box<dyn Error>> {
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
