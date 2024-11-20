use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use core::fmt;
use reqwest::Client;
use serde_json::Value;
use std::{collections::HashMap, error::Error, sync::Arc};

#[async_trait]
#[async_trait]
pub trait IPFSClient: Send + Sync {
    async fn upload(&self, file_data: Vec<u8>) -> Result<String, Box<dyn Error + Send + Sync>>;
}

#[derive(Debug)]
pub enum IPFSProvider {
    Infura,
    Pinata,
    Custom,
}

#[derive(Debug)]
struct CustomIPFSClient {
    pub api_url: String,
    pub headers: HashMap<String, String>,
}

impl fmt::Debug for dyn IPFSClient + Send + Sync {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IPFSClient + Send + Sync")
    }
}

#[async_trait]
impl IPFSClient for CustomIPFSClient {
    async fn upload(&self, file_data: Vec<u8>) -> Result<String, Box<dyn Error + Send + Sync>> {
        let client = Client::new();
        let mut request = client.post(&self.api_url);

        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        let response = request.body(file_data).send().await?;
        let response_json: Value = response.json().await?;

        let ipfs_hash = response_json["Hash"].as_str().unwrap().to_string();
        Ok(ipfs_hash)
    }
}

struct InfuraIPFSClient {
    pub project_id: String,
    pub project_secret: String,
}

#[async_trait]
impl IPFSClient for InfuraIPFSClient {
    async fn upload(&self, file_data: Vec<u8>) -> Result<String, Box<dyn Error + Send + Sync>> {
        let client = Client::new();
        let response = client
            .post("https://ipfs.infura.io:5001/api/v0/add")
            .header(
                "Authorization",
                format!(
                    "Basic {}",
                    BASE64_STANDARD.encode(format!("{}:{}", self.project_id, self.project_secret))
                ),
            )
            .body(file_data)
            .send()
            .await?;

        let response_json: Value = response.json().await?;
        let ipfs_hash = response_json["Hash"].as_str().unwrap().to_string();
        Ok(format!("{}{}", "ipfs://", ipfs_hash))
    }
}

struct PinataIPFSClient {
    pub api_key: String,
    pub secret_api_key: String,
}

#[async_trait]
impl IPFSClient for PinataIPFSClient {
    async fn upload(&self, file_data: Vec<u8>) -> Result<String, Box<dyn Error + Send + Sync>> {
        let client = Client::new();
        let response = client
            .post("https://api.pinata.cloud/pinning/pinFileToIPFS")
            .header("pinata_api_key", &self.api_key)
            .header("pinata_secret_api_key", &self.secret_api_key)
            .body(file_data)
            .send()
            .await?;

        let response_json: Value = response.json().await?;
        let ipfs_hash = response_json["IpfsHash"].as_str().unwrap().to_string();
        Ok(ipfs_hash)
    }
}

pub struct IPFSClientFactory;

impl IPFSClientFactory {
    pub fn create_client(
        provider: IPFSProvider,
        config: HashMap<String, String>,
    ) -> Result<Arc<dyn IPFSClient + Send + Sync>, Box<dyn Error + Send + Sync>> {
        match provider {
            IPFSProvider::Infura => Ok(Arc::new(InfuraIPFSClient {
                project_id: config
                    .get("project_id")
                    .ok_or("Project ID missing")?
                    .to_string(),
                project_secret: config
                    .get("project_secret")
                    .ok_or("Project Secret missing")?
                    .to_string(),
            })),
            IPFSProvider::Pinata => Ok(Arc::new(PinataIPFSClient {
                api_key: config.get("api_key").ok_or("API Key missing")?.to_string(),
                secret_api_key: config
                    .get("secret_api_key")
                    .ok_or("Secret API Key missing")?
                    .to_string(),
            })),
            IPFSProvider::Custom => {
                let api_url = config.get("api_url").ok_or("API URL missing")?.to_string();
                let headers: HashMap<String, String> = config
                    .iter()
                    .filter(|(k, _)| k != &"api_url")
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                Ok(Arc::new(CustomIPFSClient { api_url, headers }))
            }
        }
    }
}


