use crate::{
    adapters::{
        agents::{Agent, LLMModel},
        conditions::{Condition, ConditionCheck, ConditionType, TimeComparisonType},
        connectors::{
            off_chain::OffChainConnector,
            on_chain::{GasOptions, OnChainConnector, OnChainTransaction},
        },
        evaluations::{Evaluation, EvaluationType},
        fhe_gates::FHEGate,
        listeners::{Listener, ListenerType},
    },
    constants::{GRAPH_ENDPOINT_DEV, GRAPH_ENDPOINT_PROD},
    encrypt::decrypt_with_private_key,
    ipfs::{IPFSClient, IPFSClientFactory, IPFSProvider},
    nibble::ContractInfo,
    workflow::{LinkAdapter, NodeAdapter, WorkflowLink, WorkflowNode},
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Utc;
use dotenv::dotenv;
use ethers::{
    abi::Token,
    signers::LocalWallet,
    types::{Address, H160, U256},
    utils::hex,
};
use rand::Rng;
use reqwest::{Client, Method};
use serde_json::{from_value, json, Map, Value};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, env, error::Error, iter::Iterator, str::FromStr, sync::Arc};
use tokio::time::Duration;

pub struct GraphWorkflowResponse {
    pub id: Vec<u8>,
    pub name: String,
    pub nodes: Vec<WorkflowNode>,
    pub links: Vec<WorkflowLink>,
    pub dependent_workflows: Vec<Vec<u8>>,
    pub encrypted: bool,
}

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

pub fn convert_value_to_token(value: &Value) -> Result<Token, Box<dyn Error + Send + Sync>> {
    match value {
        Value::Number(num) if num.is_u64() => Ok(Token::Uint(U256::from(num.as_u64().unwrap()))),
        Value::String(s) => Ok(Token::String(s.clone())),
        _ => Err("Unsupported parameter type".into()),
    }
}

pub fn load_ipfs_client() -> Result<Arc<dyn IPFSClient + Send + Sync>, Box<dyn Error + Send + Sync>>
{
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

pub fn generate_unique_id(address: &H160) -> Vec<u8> {
    let timestamp = Utc::now().timestamp_nanos_opt().expect("Invalid timestamp");

    let random_bytes: [u8; 4] = rand::thread_rng().gen();

    let mut hasher = Sha256::new();
    hasher.update(address.as_bytes());
    let address_hash = hasher.finalize();

    let mut unique_id = Vec::with_capacity(12 + address_hash.len());
    unique_id.extend_from_slice(&timestamp.to_be_bytes());
    unique_id.extend_from_slice(&random_bytes);
    unique_id.extend_from_slice(&address_hash[..8]);

    unique_id
}

pub async fn load_workflow_from_subgraph(
    workflow_id: Vec<u8>,
    nibble_id: Vec<u8>,
    api_key: Option<String>,
) -> Result<GraphWorkflowResponse, Box<dyn Error + Send + Sync>> {
    let mut url = GRAPH_ENDPOINT_DEV.to_string();

    if api_key.is_some() {
        url = GRAPH_ENDPOINT_PROD.replace("apikey", &api_key.unwrap());
    }

    let client = Client::new();

    let query = json!({
        "query": r#"
                    query Workflow($id: ID!, $nibble_id: nibble_id) {
                        workflow(id: $id, nibble_id: $nibble_id) {
                            id
                            name
                            nodes
                            links
                        }
                    }
                "#,
        "variables": {
            "id": String::from_utf8(workflow_id)?,
            "nibble_id": String::from_utf8(nibble_id)?
        }
    });
    let res = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&query)
        .send()
        .await?;

    if res.status().is_success() {
        let json: serde_json::Value = res.json().await?;

        if let Some(object) = json["data"]["nibble"].as_object() {
            let id = object
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or("Missing or invalid id")?
                .to_string();

            return Ok(GraphWorkflowResponse {
                id: STANDARD.decode(&id).map_err(|_| "Failed to decode id")?,
                name: object
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing name")?
                    .parse::<String>()?,
                encrypted: object
                    .get("encrpyted")
                    .and_then(|v| v.as_bool())
                    .ok_or("Missing encrpyted")?,
                nodes: build_nodes(object.get("nodes").unwrap())?,
                links: build_links(object.get("links").unwrap())?,
                dependent_workflows: object
                    .get("dependent_workflows")
                    .and_then(|v| v.as_array())
                    .map(|array| {
                        array
                            .iter()
                            .map(|item| {
                                item.as_array()
                                    .map(|nested| {
                                        nested
                                            .iter()
                                            .filter_map(|val| val.as_u64())
                                            .flat_map(|u| u.to_be_bytes().to_vec())
                                            .collect::<Vec<u8>>()
                                    })
                                    .unwrap_or_default()
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            });
        } else {
            return Err("No data returned from Graph query".into());
        }
    } else {
        let error_text = res.text().await?;
        Err(error_text.into())
    }
}

pub async fn load_nibble_from_subgraph(
    id: Vec<u8>,
    api_key: Option<String>,
    wallet: LocalWallet,
) -> Result<GraphNibbleResponse, Box<dyn Error + Send + Sync>> {
    let mut url = GRAPH_ENDPOINT_DEV.to_string();

    if api_key.is_some() {
        url = GRAPH_ENDPOINT_PROD.replace("apikey", &api_key.unwrap());
    }

    let client = Client::new();

    let query = json!({
        "query": r#"
                query Nibble($id: ID!) {
                    nibble(id: $id) {
                        agents {
                            id
                            name
                        }
                        conditions
                        listeners
                        fhe_gates
                        evaluations
                        onchain_connectors
                        offchain_connectors
                        contracts
                        count
                    }
                }
            "#,
        "variables": {
            "id": String::from_utf8(id)?
        }
    });
    let res = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&query)
        .send()
        .await?;

    if res.status().is_success() {
        let json: serde_json::Value = res.json().await?;

        if let Some(object) = json["data"]["nibble"].as_object() {
            return Ok(GraphNibbleResponse {
                agents: build_agents(object.get("agents").unwrap(), wallet.clone()).await?,
                conditions: build_conditions(object.get("conditions").unwrap(), wallet.clone())
                    .await?,
                listeners: build_listeners(object.get("listeners").unwrap(), wallet.clone())
                    .await?,
                fhe_gates: build_fhe_gates(object.get("fhe_gates").unwrap(), wallet.clone())
                    .await?,
                evaluations: build_evaluations(object.get("evaluations").unwrap(), wallet.clone())
                    .await?,
                onchain_connectors: build_onchain_connectors(
                    object.get("onchain_connectors").unwrap(),
                    wallet.clone(),
                )
                .await?,
                offchain_connectors: build_offchain_connectors(
                    object.get("offchain_connectors").unwrap(),
                    wallet.clone(),
                )
                .await?,
                contracts: object
                    .get("contracts")
                    .cloned()
                    .ok_or("Missing contracts")?
                    .as_array()
                    .ok_or("Contracts should be an array")?
                    .iter()
                    .map(|v| from_value(v.clone()))
                    .collect::<Result<_, _>>()?,
                count: object
                    .get("count")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing count")?
                    .parse::<U256>()?,
            });
        } else {
            return Err("No data returned from Graph query".into());
        }
    } else {
        let error_text = res.text().await?;
        Err(error_text.into())
    }
}

async fn fetch_metadata_from_ipfs(
    metadata_hash: &str,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let ipfs_url = format!("https://thedial.infura-ipfs.io/ipfs/{}", metadata_hash);
    let client = Client::new();
    let res = client.get(&ipfs_url).send().await?;
    let metadata: Value = res.json().await?;
    Ok(metadata)
}

async fn build_agents(
    data: &Value,
    wallet: LocalWallet,
) -> Result<Vec<Agent>, Box<dyn Error + Send + Sync>> {
    let mut agents = Vec::new();
    if let Some(agent_array) = data.as_array() {
        for agent_data in agent_array {
            let id = agent_data
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| hex::decode(s).unwrap_or_default())
                .unwrap_or_default();

            let metadata_hash = agent_data
                .get("metadata")
                .and_then(|v| v.as_str())
                .ok_or("Missing metadata")?;

            let address = agent_data
                .get("wallet")
                .and_then(|v| v.as_str())
                .ok_or("Missing wallet")?
                .to_string();

            let encrypted = agent_data
                .get("encrypted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let write_role = agent_data
                .get("writer")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let admin_role = agent_data
                .get("admin")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let mut metadata = fetch_metadata_from_ipfs(metadata_hash).await?;

            if encrypted {
                let metadata_bytes = serde_json::to_vec(&metadata)?;
                metadata = decrypt_with_private_key(metadata_bytes, wallet.clone())?;
            }

            let role = metadata
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let personality = metadata
                .get("personality")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let system = metadata
                .get("system")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let model = parse_llm_model(&metadata)?;
            let lens_account = metadata
                .get("lens_account")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let farcaster_account = metadata
                .get("farcaster_account")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            agents.push(Agent {
                name: metadata
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                id,
                role,
                personality,
                system,
                model,
                encrypted,
                wallet: LocalWallet::from_str(&address)?,
                write_role,
                admin_role,
                farcaster_account: Some(farcaster_account),
                lens_account: Some(lens_account),
            });
        }
    }
    Ok(agents)
}

fn parse_llm_model(metadata: &Value) -> Result<LLMModel, Box<dyn Error + Send + Sync>> {
    let model_type = metadata
        .get("model_type")
        .and_then(|v| v.as_str())
        .ok_or("Missing model_type")?;

    match model_type {
        "OpenAI" => Ok(LLMModel::OpenAI {
            api_key: metadata
                .get("api_key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            model: metadata
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            temperature: metadata
                .get("temperature")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32,
            max_tokens: metadata
                .get("max_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(1000) as u32,
            top_p: metadata
                .get("top_p")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32,
            frequency_penalty: metadata
                .get("frequency_penalty")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32,
            presence_penalty: metadata
                .get("presence_penalty")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32,
            system_prompt: metadata
                .get("system_prompt")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        }),
        "Claude" => Ok(LLMModel::Claude {
            api_key: metadata
                .get("api_key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            model: metadata
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            temperature: metadata
                .get("temperature")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32,
            max_tokens: metadata
                .get("max_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(1000) as u32,
            top_k: metadata
                .get("top_k")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            top_p: metadata
                .get("top_p")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32,
            system_prompt: metadata
                .get("system_prompt")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        }),
        "Ollama" => Ok(LLMModel::Ollama {
            api_key: metadata
                .get("api_key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            model: metadata
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            temperature: metadata
                .get("temperature")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32,
            max_tokens: metadata
                .get("max_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(1000) as u32,
            top_p: metadata
                .get("top_p")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32,
            frequency_penalty: metadata
                .get("frequency_penalty")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32,
            presence_penalty: metadata
                .get("presence_penalty")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32,
        }),
        _ => Ok(LLMModel::Other {
            config: metadata
                .as_object()
                .unwrap_or(&Map::new())
                .iter()
                .map(|(k, v)| (k.clone(), v.to_string()))
                .collect(),
        }),
    }
}

async fn build_conditions(
    data: &Value,
    wallet: LocalWallet,
) -> Result<Vec<Condition>, Box<dyn Error + Send + Sync>> {
    let mut conditions = Vec::new();

    if let Some(condition_array) = data.as_array() {
        for condition_data in condition_array {
            let id = condition_data
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| hex::decode(s).unwrap_or_default())
                .unwrap_or_default();

            let metadata_hash = condition_data
                .get("metadata")
                .and_then(|v| v.as_str())
                .ok_or("Missing metadata")?;

            let encrypted = condition_data
                .get("encrypted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let mut metadata = fetch_metadata_from_ipfs(metadata_hash).await?;

            if encrypted {
                let metadata_bytes = serde_json::to_vec(&metadata)?;
                metadata = decrypt_with_private_key(metadata_bytes, wallet.clone())?;
            }

            let name = metadata
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed Condition")
                .to_string();

            let condition_type = match metadata
                .get("condition_type")
                .and_then(|v| v.as_str())
                .ok_or("Missing condition_type")?
            {
                "OnChain" => ConditionType::OnChain {
                    contract_address: metadata
                        .get("contract_address")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing contract_address")?
                        .parse::<Address>()?,
                    function_signature: metadata
                        .get("function_signature")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                },
                "OffChain" => ConditionType::OffChain {
                    api_url: metadata
                        .get("api_url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                },
                "InternalState" => ConditionType::InternalState {
                    field_name: metadata
                        .get("field_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                },
                "ContextBased" => ConditionType::ContextBased {
                    key: metadata
                        .get("key")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                },
                "TimeBased" => ConditionType::TimeBased {
                    comparison_time: metadata
                        .get("comparison_time")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing comparison_time")?
                        .parse::<chrono::NaiveTime>()?,
                    comparison_type: match metadata
                        .get("comparison_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("After")
                    {
                        "Before" => TimeComparisonType::Before,
                        "After" => TimeComparisonType::After,
                        _ => return Err("Invalid comparison_type".into()),
                    },
                },
                _ => return Err("Invalid condition_type".into()),
            };

            let check = ConditionCheck {
                condition_fn: |_v| true,
                expected_value: metadata.get("expected_value").cloned(),
            };

            conditions.push(Condition {
                name,
                condition_type,
                check,
                encrypted,
                id,
            });
        }
    }

    Ok(conditions)
}

async fn build_listeners(
    data: &Value,
    wallet: LocalWallet,
) -> Result<Vec<Listener>, Box<dyn Error + Send + Sync>> {
    let mut listeners = Vec::new();

    if let Some(listener_array) = data.as_array() {
        for listener_data in listener_array {
            let id = listener_data
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| hex::decode(s).unwrap_or_default())
                .unwrap_or_default();

            let metadata_hash = listener_data
                .get("metadata")
                .and_then(|v| v.as_str())
                .ok_or("Missing metadata")?;

            let encrypted = listener_data
                .get("encrypted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let mut metadata = fetch_metadata_from_ipfs(metadata_hash).await?;

            if encrypted {
                let metadata_bytes = serde_json::to_vec(&metadata)?;
                metadata = decrypt_with_private_key(metadata_bytes, wallet.clone())?;
            }

            let name = metadata
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed Listener")
                .to_string();

            let listener_type = match metadata
                .get("listener_type")
                .and_then(|v| v.as_str())
                .ok_or("Missing listener_type")?
            {
                "OnChain" => ListenerType::OnChain {
                    contract_address: metadata
                        .get("contract_address")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing contract_address")?
                        .parse::<Address>()?,
                    event_signature: metadata
                        .get("event_signature")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                },
                "OffChain" => ListenerType::OffChain {
                    webhook_url: metadata
                        .get("webhook_url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                },
                "Timer" => ListenerType::Timer {
                    interval: metadata
                        .get("interval")
                        .and_then(|v| v.as_u64())
                        .map(Duration::from_secs)
                        .ok_or("Missing interval")?,
                    check_onchain: metadata
                        .get("check_onchain")
                        .and_then(|v| v.as_str())
                        .map(|s| s.parse::<Address>())
                        .transpose()?,
                    check_offchain: metadata
                        .get("check_offchain")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                },
                _ => return Err("Invalid listener_type".into()),
            };

            let condition = ConditionCheck {
                condition_fn: |_v| true,
                expected_value: metadata.get("expected_value").cloned(),
            };

            listeners.push(Listener {
                name,
                id,
                event_name: metadata
                    .get("event_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unnamed Event")
                    .to_string(),
                listener_type,
                condition,
                encrypted,
            });
        }
    }

    Ok(listeners)
}

async fn build_evaluations(
    data: &Value,
    wallet: LocalWallet,
) -> Result<Vec<Evaluation>, Box<dyn Error + Send + Sync>> {
    let mut evaluations = Vec::new();

    if let Some(evaluation_array) = data.as_array() {
        for evaluation_data in evaluation_array {
            let id = evaluation_data
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| hex::decode(s).unwrap_or_default())
                .unwrap_or_default();

            let metadata_hash = evaluation_data
                .get("metadata")
                .and_then(|v| v.as_str())
                .ok_or("Missing metadata")?;

            let encrypted: bool = evaluation_data
                .get("encrypted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let mut metadata = fetch_metadata_from_ipfs(metadata_hash).await?;

            if encrypted {
                let metadata_bytes = serde_json::to_vec(&metadata)?;
                metadata = decrypt_with_private_key(metadata_bytes, wallet.clone())?;
            }

            let name = metadata
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed Evaluation")
                .to_string();

            let evaluation_type = match metadata
                .get("evaluation_type")
                .and_then(|v| v.as_str())
                .ok_or("Missing evaluation_type")?
            {
                "HumanJudge" => EvaluationType::HumanJudge {
                    prompt: metadata
                        .get("prompt")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    approval_required: metadata
                        .get("approval_required")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                },
                "LLMJudge" => EvaluationType::LLMJudge {
                    model_name: metadata
                        .get("model_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    prompt_template: metadata
                        .get("prompt_template")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    approval_threshold: metadata
                        .get("approval_threshold")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                },
                "ContextualJudge" => EvaluationType::ContextualJudge {
                    context_fn: Arc::new(|_value: Value| true),
                },
                _ => return Err("Invalid evaluation_type".into()),
            };

            evaluations.push(Evaluation {
                name,
                encrypted,
                id,
                evaluation_type,
            });
        }
    }

    Ok(evaluations)
}

async fn build_fhe_gates(
    data: &Value,
    wallet: LocalWallet,
) -> Result<Vec<FHEGate>, Box<dyn Error + Send + Sync>> {
    let mut fhe_gates = Vec::new();

    if let Some(fhe_gate_array) = data.as_array() {
        for fhe_gate_data in fhe_gate_array {
            let id = fhe_gate_data
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| hex::decode(s).unwrap_or_default())
                .unwrap_or_default();

            let metadata_hash = fhe_gate_data
                .get("metadata")
                .and_then(|v| v.as_str())
                .ok_or("Missing metadata")?;

            let encrypted = fhe_gate_data
                .get("encrypted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let mut metadata = fetch_metadata_from_ipfs(metadata_hash).await?;

            if encrypted {
                let metadata_bytes = serde_json::to_vec(&metadata)?;
                metadata = decrypt_with_private_key(metadata_bytes, wallet.clone())?;
            }
            let name = metadata
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed FHE Gate")
                .to_string();

            let key = metadata
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            fhe_gates.push(FHEGate {
                name,
                id,
                key,
                encrypted,
            });
        }
    }

    Ok(fhe_gates)
}

async fn build_onchain_connectors(
    data: &Value,
    wallet: LocalWallet,
) -> Result<Vec<OnChainConnector>, Box<dyn Error + Send + Sync>> {
    let mut onchain_connectors = Vec::new();

    if let Some(connector_array) = data.as_array() {
        for connector_data in connector_array {
            let id = connector_data
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| hex::decode(s).unwrap_or_default())
                .unwrap_or_default();

            let metadata_hash = connector_data
                .get("metadata")
                .and_then(|v| v.as_str())
                .ok_or("Missing metadata")?;

            let encrypted = connector_data
                .get("encrypted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let is_onchain = connector_data
                .get("onChain")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !is_onchain {
                continue;
            }

            let mut metadata = fetch_metadata_from_ipfs(metadata_hash).await?;

            if encrypted {
                let metadata_bytes = serde_json::to_vec(&metadata)?;
                metadata = decrypt_with_private_key(metadata_bytes, wallet.clone())?;
            }

            let name = metadata
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed OnChain Connector")
                .to_string();

            let address = metadata
                .get("address")
                .and_then(|v| v.as_str())
                .ok_or("Missing address")?
                .parse::<Address>()?;

            let transactions =
                if let Some(tx_array) = metadata.get("transactions").and_then(|v| v.as_array()) {
                    tx_array
                        .iter()
                        .filter_map(|tx| {
                            Some(OnChainTransaction {
                                function_signature: tx
                                    .get("function_signature")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                params: tx
                                    .get("params")
                                    .and_then(|v| v.as_array())
                                    .cloned()
                                    .unwrap_or_else(Vec::new),
                                gas_options: GasOptions {
                                    max_fee_per_gas: tx
                                        .get("max_fee_per_gas")
                                        .and_then(|v| v.as_str())
                                        .and_then(|s| s.parse::<U256>().ok()),
                                    max_priority_fee_per_gas: tx
                                        .get("max_priority_fee_per_gas")
                                        .and_then(|v| v.as_str())
                                        .and_then(|s| s.parse::<U256>().ok()),
                                    gas_limit: tx
                                        .get("gas_limit")
                                        .and_then(|v| v.as_str())
                                        .and_then(|s| s.parse::<U256>().ok()),
                                    nonce: tx
                                        .get("nonce")
                                        .and_then(|v| v.as_str())
                                        .and_then(|s| s.parse::<U256>().ok()),
                                },
                            })
                        })
                        .collect()
                } else {
                    Vec::new()
                };

            onchain_connectors.push(OnChainConnector {
                name,
                id,
                address,
                encrypted,
                transactions,
            });
        }
    }

    Ok(onchain_connectors)
}

pub async fn build_offchain_connectors(
    data: &Value,
    wallet: LocalWallet,
) -> Result<Vec<OffChainConnector>, Box<dyn Error + Send + Sync>> {
    let mut offchain_connectors = Vec::new();

    if let Some(connector_array) = data.as_array() {
        for connector_data in connector_array {
            let id = connector_data
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| hex::decode(s).unwrap_or_default())
                .unwrap_or_default();

            let metadata_hash = connector_data
                .get("metadata")
                .and_then(|v| v.as_str())
                .ok_or("Missing metadata")?;

            let encrypted = connector_data
                .get("encrypted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let is_onchain = connector_data
                .get("onChain")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if is_onchain {
                continue;
            }

            let mut metadata = fetch_metadata_from_ipfs(metadata_hash).await?;

            if encrypted {
                let metadata_bytes = serde_json::to_vec(&metadata)?;
                metadata = decrypt_with_private_key(metadata_bytes, wallet.clone())?;
            }

            let name = metadata
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed OffChain Connector")
                .to_string();

            let api_url = metadata
                .get("api_url")
                .and_then(|v| v.as_str())
                .ok_or("Missing api_url")?
                .to_string();

            let http_method = metadata
                .get("http_method")
                .and_then(|v| v.as_str())
                .map(|s| match s {
                    "GET" => Method::GET,
                    "POST" => Method::POST,
                    "PUT" => Method::PUT,
                    "DELETE" => Method::DELETE,
                    _ => Method::GET,
                })
                .unwrap_or(Method::GET);

            let headers = metadata
                .get("headers")
                .and_then(|v| v.as_object())
                .map(|map| {
                    map.iter()
                        .filter_map(|(k, v)| v.as_str().map(|val| (k.clone(), val.to_string())))
                        .collect::<HashMap<String, String>>()
                });

            let execution_fn: Option<
                Arc<dyn Fn(Value) -> Result<Value, Box<dyn Error + Send + Sync>> + Send + Sync>,
            > = Some(Arc::new(|_input: Value| Ok(Value::Null)));

            offchain_connectors.push(OffChainConnector {
                name,
                id,
                api_url,
                encrypted,
                http_method,
                headers,
                execution_fn,
            });
        }
    }

    Ok(offchain_connectors)
}

fn build_nodes(data: &Value) -> Result<Vec<WorkflowNode>, Box<dyn Error + Send + Sync>> {
    let mut nodes = Vec::new();

    if let Some(node_array) = data.as_array() {
        for node_data in node_array {
            let id = node_data
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| hex::decode(s).unwrap_or_default())
                .unwrap_or_default();

            let adapter_type = match node_data
                .get("adapter_type")
                .and_then(|v| v.as_str())
                .ok_or("Missing adapter_type")?
            {
                "OffChainConnector" => NodeAdapter::OffChainConnector,
                "OnChainConnector" => NodeAdapter::OnChainConnector,
                "Agent" => NodeAdapter::Agent,
                _ => return Err("Invalid adapter_type".into()),
            };

            let adapter_id = node_data
                .get("adapter_id")
                .and_then(|v| v.as_str())
                .map(|s| hex::decode(s).unwrap_or_default())
                .unwrap_or_default();

            nodes.push(WorkflowNode {
                id,
                adapter_type,
                adapter_id,
            });
        }
    }

    Ok(nodes)
}

fn build_links(data: &Value) -> Result<Vec<WorkflowLink>, Box<dyn Error + Send + Sync>> {
    let mut links = Vec::new();

    if let Some(link_array) = data.as_array() {
        for link_data in link_array {
            let from_node: Vec<u8> = link_data
                .get("from_node")
                .and_then(|v| v.as_str())
                .map(|s| hex::decode(s).unwrap_or_default())
                .unwrap_or_default();

            let to_node = link_data
                .get("to_node")
                .and_then(|v| v.as_str())
                .map(|s| hex::decode(s).unwrap_or_default())
                .unwrap_or_default();

            let id = link_data
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| hex::decode(s).unwrap_or_default())
                .unwrap_or_default();

            let adapter_id = link_data
                .get("adapter_id")
                .and_then(|v| v.as_str())
                .map(|s| hex::decode(s).unwrap_or_default())
                .unwrap_or_default();

            let adapter_type = match link_data
                .get("adapter_type")
                .and_then(|v| v.as_str())
                .ok_or("Missing adapter_type")?
            {
                "Evaluation" => LinkAdapter::Evaluation,
                "Condition" => LinkAdapter::Condition,
                "FHEGate" => LinkAdapter::FHEGate,
                "Listener" => LinkAdapter::Listener,

                _ => return Err("Invalid adapter_type".into()),
            };

            links.push(WorkflowLink {
                id,
                adapter_id,
                adapter_type,
                from_node,
                to_node,
            });
        }
    }

    Ok(links)
}
