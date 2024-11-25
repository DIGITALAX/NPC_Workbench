use crate::{
    adapters::{
        links::{
            conditions::{Condition, ConditionCheck, ConditionType, TimeComparisonType},
            evaluations::{Evaluation, EvaluationResponseType, EvaluationType},
            fhe_gates::FHEGate,
            listeners::{Listener, ListenerType},
        },
        nodes::{
            agents::{Agent, LLMModel, Objective},
            connectors::{
                off_chain::{ConnectorType, OffChainConnector},
                on_chain::{GasOptions, OnChainConnector, OnChainTransaction},
            },
        },
    },
    constants::{GRAPH_ENDPOINT_DEV, GRAPH_ENDPOINT_PROD},
    encrypt::decrypt_with_private_key,
    nibble::ContractInfo,
    tools::{context::ContextParse, history::HistoryParse},
    workflow::{
        ExecutionHistory, LinkAdapter, LinkTarget, NodeAdapter, WorkflowLink, WorkflowNode,
    },
};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{DateTime, Utc};
use ethers::{
    abi::Token,
    providers::{Http, Provider},
    signers::LocalWallet,
    types::{Address, Chain, H160, U256},
    utils::hex,
};
use rand::Rng;
use reqwest::{Client, Method};
use serde_json::{from_value, json, Map, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap, convert::TryFrom, error::Error, iter::Iterator, str::FromStr, sync::Arc,
};
use tokio::time::Duration;

pub struct GraphWorkflowResponse {
    pub id: Vec<u8>,
    pub name: String,
    pub nodes: HashMap<Vec<u8>, WorkflowNode>,
    pub links: HashMap<Vec<u8>, WorkflowLink>,
    pub encrypted: bool,
    pub execution_history: Vec<ExecutionHistory>,
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
    pub debug: bool,
}

pub fn convert_value_to_token(value: &Value) -> Result<Token, Box<dyn Error + Send + Sync>> {
    match value {
        Value::Number(num) if num.is_u64() => Ok(Token::Uint(U256::from(num.as_u64().unwrap()))),
        Value::String(s) => Ok(Token::String(s.clone())),
        _ => Err("Unsupported parameter type".into()),
    }
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
                    .get("encrypted")
                    .and_then(|v| v.as_bool())
                    .ok_or("Missing encrypted")?,
                nodes: build_nodes(object.get("nodes").unwrap())?,
                links: build_links(object.get("links").unwrap())?,
                execution_history: build_execution_history(
                    object.get("execution_history").unwrap(),
                )?,
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
    provider: Provider<Http>,
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
                listeners: build_listeners(
                    object.get("listeners").unwrap(),
                    wallet.clone(),
                    provider,
                )
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
                debug: object
                    .get("debug")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
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

            let objectives = metadata
                .get("objectives")
                .and_then(|v| v.as_array())
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| Objective::try_from(v).ok())
                .collect();

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
                objectives,
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
            max_completion_tokens: metadata
                .get("max_completion_tokens")
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
            store: metadata.get("store").and_then(|v| v.as_bool()),
            metadata: metadata.get("metadata").cloned(),
            logit_bias: metadata.get("logit_bias").cloned(),
            logprobs: metadata.get("logprobs").and_then(|v| v.as_bool()),
            top_logprobs: metadata
                .get("top_logprobs")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            modalities: metadata
                .get("modalities")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| m.as_str().map(|s| s.to_string()))
                        .collect()
                }),
            stop: metadata.get("stop").and_then(|v| v.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str().map(|s| s.to_string()))
                    .collect()
            }),
            response_format: metadata.get("response_format").cloned(),
            stream: metadata.get("stream").and_then(|v| v.as_bool()),
            parallel_tool_calls: metadata
                .get("parallel_tool_calls")
                .and_then(|v| v.as_bool()),
            user: metadata
                .get("user")
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
            version: metadata
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            stop_sequences: metadata
                .get("stop_sequences")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(|s| s.to_string()))
                        .collect()
                }),
            stream: metadata
                .get("stream")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            metadata: metadata.get("metadata").cloned(),
            tool_choice: metadata.get("tool_choice").cloned(),
            tools: metadata
                .get("tools")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().cloned().collect()),
        }),
        "Ollama" => Ok(LLMModel::Ollama {
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
            format: metadata
                .get("format")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            suffix: metadata
                .get("suffix")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            system: metadata
                .get("system")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            template: metadata
                .get("template")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            context: metadata
                .get("context")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_u64().map(|n| n as u32))
                        .collect()
                }),
            stream: metadata.get("stream").and_then(|v| v.as_bool()),
            raw: metadata.get("raw").and_then(|v| v.as_bool()),
            keep_alive: metadata
                .get("keep_alive")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            options: metadata.get("options").cloned(),
            images: metadata
                .get("images")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|img| img.as_str().map(|s| s.to_string()))
                        .collect()
                }),
        }),
        _ => Ok(LLMModel::Other {
            url: metadata
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            result_path: metadata
                .get("result_path")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            result_type: metadata
                .get("result_type")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            api_key: metadata
                .get("api_key")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            body: metadata
                .get("body")
                .and_then(|v| v.as_object())
                .unwrap_or(&Map::new())
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
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

                "ContextBased" => ConditionType::ContextBased {},
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
    provider: Provider<Http>,
) -> Result<Vec<Listener>, Box<dyn Error + Send + Sync>> {
    let mut listeners = Vec::new();

    if let Some(listener_array) = data.as_array() {
        for listener_data in listener_array {
            let provider = provider.clone();
            let wallet = wallet.clone();
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
                    abi: metadata
                        .get("abi")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing abi")?
                        .to_string(),

                    chain: metadata
                        .get("chain")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing chain")?
                        .parse::<Chain>()?,
                    provider,
                    wallet,
                },
                "OffChain" => ListenerType::OffChain {
                    webhook_url: metadata
                        .get("webhook_url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    sns_verification: metadata
                        .get("sns_verification")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                },
                "Timer" => ListenerType::Timer {
                    interval: metadata
                        .get("interval")
                        .and_then(|v| v.as_u64())
                        .map(Duration::from_secs)
                        .ok_or("Missing interval")?,
                },
                _ => return Err("Invalid listener_type".into()),
            };

            listeners.push(Listener {
                name,
                id,
                listener_type,
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
                    timeout: metadata
                        .get("timeout")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse::<u64>().ok())
                        .map(|secs| Duration::from_secs(secs))
                        .unwrap_or_else(|| Duration::from_secs(0)),
                    default: metadata
                        .get("default")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    endpoint: metadata
                        .get("endpoint")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    auth_key: Some(
                        metadata
                            .get("auth_key")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    ),
                },
                "LLMJudge" => EvaluationType::LLMJudge {
                    model_type: parse_llm_model(&metadata)?,
                    prompt: metadata
                        .get("prompt")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    response_type: match metadata.get("response_type") {
                        Some(Value::Bool(expected)) => EvaluationResponseType::Boolean {
                            expected: *expected,
                        },
                        Some(Value::Number(num)) => num
                            .as_f64()
                            .map(|threshold| EvaluationResponseType::Score { threshold })
                            .unwrap_or(EvaluationResponseType::Dynamic),
                        Some(_) => EvaluationResponseType::Dynamic,
                        None => EvaluationResponseType::Dynamic,
                    },
                },
                "AgentJudge" => EvaluationType::AgentJudge {
                    prompt: metadata
                        .get("prompt")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    agent_id: metadata
                        .get("agent_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.as_bytes().to_vec())
                        .unwrap_or_else(|| vec![0]),
                    response_type: match metadata.get("response_type") {
                        Some(Value::Bool(expected)) => EvaluationResponseType::Boolean {
                            expected: *expected,
                        },
                        Some(Value::Number(num)) => num
                            .as_f64()
                            .map(|threshold| EvaluationResponseType::Score { threshold })
                            .unwrap_or(EvaluationResponseType::Dynamic),
                        Some(_) => EvaluationResponseType::Dynamic,
                        None => EvaluationResponseType::Dynamic,
                    },
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

            let contract_address = fhe_gate_data
                .get("contract_address")
                .and_then(|v| v.as_array())
                .and_then(|arr| {
                    if arr.len() == 20 {
                        Some(H160::from_slice(
                            &arr.iter()
                                .filter_map(|v| v.as_u64().map(|x| x as u8))
                                .collect::<Vec<_>>(),
                        ))
                    } else {
                        None
                    }
                })
                .ok_or("Invalid or missing contract address")?;

            let operation = fhe_gate_data
                .get("operation")
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed Operation")
                .to_string();

            let chain = fhe_gate_data
                .get("chain")
                .and_then(|s| s.as_str())
                .and_then(|s| s.parse::<Chain>().ok())
                .unwrap_or(Chain::Mainnet);

            fhe_gates.push(FHEGate {
                name,
                id,
                key,
                encrypted,
                contract_address,
                operation,
                chain,
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
                                chain: tx
                                    .get("chain")
                                    .and_then(|s| s.as_str())
                                    .and_then(|s| s.parse::<Chain>().ok())
                                    .unwrap_or(Chain::Mainnet),
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

            let connector_type = metadata
                .get("connector_type")
                .and_then(|v| v.as_str())
                .ok_or("Missing connector_type")?;

            let connector_type = match connector_type {
                "REST" => {
                    let base_payload = metadata.get("base_payload").cloned().unwrap_or(Value::Null);
                    ConnectorType::REST {
                        base_payload: if base_payload.is_null() {
                            None
                        } else {
                            Some(base_payload)
                        },
                    }
                }
                "GraphQL" => {
                    let query = metadata
                        .get("query")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing query for GraphQL connector")?
                        .to_string();

                    let variables =
                        metadata
                            .get("variables")
                            .and_then(|v| v.as_object())
                            .map(|map| {
                                map.iter()
                                    .filter_map(|(k, v)| {
                                        v.as_str().map(|val| (k.clone(), val.to_string()))
                                    })
                                    .collect::<HashMap<String, String>>()
                            });

                    ConnectorType::GraphQL { query, variables }
                }
                _ => return Err("Invalid connector_type".into()),
            };

            let execution_fn: Option<
                Arc<dyn Fn(Value) -> Result<Value, Box<dyn Error + Send + Sync>> + Send + Sync>,
            > = Some(Arc::new(|_input: Value| Ok(Value::Null)));

            offchain_connectors.push(OffChainConnector {
                name,
                id,
                connector_type,
                api_url,
                encrypted,
                http_method,
                headers,
                params: None,
                auth_tokens: None,
                result_processing_fn: execution_fn,
                auth_subflow: None,
            });
        }
    }

    Ok(offchain_connectors)
}

fn build_nodes(
    data: &Value,
) -> Result<HashMap<Vec<u8>, WorkflowNode>, Box<dyn Error + Send + Sync>> {
    let mut nodes = HashMap::new();

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

            let repetitions = node_data
                .get("repetitions")
                .and_then(|v| v.as_u64())
                .and_then(|val| u32::try_from(val).ok());

            let context: Option<Value> = node_data
                .get("context")
                .and_then(|val| Value::try_from(val.clone()).ok());

            let description = node_data
                .get("description")
                .and_then(|val| val.as_str().map(|s| s.to_string()));

            let context_tool = node_data
                .get("context_tool")
                .and_then(|v| v.as_object())
                .map(|tool_data| {
                    let required_fields: Vec<String> = tool_data
                        .get("required_fields")
                        .and_then(|fields| fields.as_array())
                        .map(|fields| {
                            fields
                                .iter()
                                .filter_map(|field| field.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    ContextParse::ParseFields {
                        expected_format: tool_data.clone(),
                        required_fields,
                    }
                });

            let history_tool = node_data
                .get("history_tool")
                .and_then(|v| v.as_object())
                .map(|tool_data| {
                    if let Some(index) = tool_data.get("index").and_then(|v| v.as_u64()) {
                        let field_path: Vec<String> = tool_data
                            .get("field_path")
                            .and_then(|fields| fields.as_array())
                            .map(|fields| {
                                fields
                                    .iter()
                                    .filter_map(|field| field.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();

                        HistoryParse::ExtractField {
                            index: index as usize,
                            field_path,
                        }
                    } else {
                        eprintln!("Invalid or missing 'index' in history_tool configuration.");
                        HistoryParse::CustomProcessor {
                            function: |_| Err("Invalid history_tool configuration".to_string()),
                        }
                    }
                });

            nodes.insert(
                id.clone(),
                WorkflowNode {
                    id,
                    adapter_type,
                    adapter_id,
                    repetitions,
                    context,
                    description,
                    history_tool,
                    context_tool,
                },
            );
        }
    }

    Ok(nodes)
}

fn build_execution_history(
    data: &Value,
) -> Result<Vec<ExecutionHistory>, Box<dyn Error + Send + Sync>> {
    let mut execution_history = Vec::new();

    if let Some(history_array) = data.as_array() {
        for item in history_array {
            let element_id = item
                .get("element_id")
                .and_then(|v| v.as_str())
                .map(|s| hex::decode(s).unwrap_or_default())
                .unwrap_or_default();

            let element_type = item
                .get("element_type")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();

            let result = item.get("result").cloned();

            let timestamp = item
                .get("timestamp")
                .and_then(|v| v.as_str())
                .and_then(|ts| DateTime::<Utc>::from_str(ts).ok())
                .unwrap_or_else(Utc::now);
            let description = item
                .get("description")
                .and_then(|val| val.as_str().map(|s| s.to_string()));

            execution_history.push(ExecutionHistory {
                element_id,
                element_type,
                result,
                timestamp,
                description,
            });
        }
    }

    Ok(execution_history)
}

fn build_links(
    data: &Value,
) -> Result<HashMap<Vec<u8>, WorkflowLink>, Box<dyn Error + Send + Sync>> {
    let mut links = HashMap::new();

    if let Some(link_array) = data.as_array() {
        for link_data in link_array {
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

            let repetitions = link_data
                .get("repetitions")
                .and_then(|v| v.as_u64())
                .and_then(|val| u32::try_from(val).ok());

            let context = link_data
                .get("context")
                .and_then(|val| Value::try_from(val.clone()).ok());

            let description = link_data
                .get("description")
                .and_then(|val| val.as_str().map(|s| s.to_string()));

            let target = link_data
                .get("target")
                .and_then(|v| v.as_str())
                .map(|s| hex::decode(s).unwrap_or_default())
                .map(|decoded| LinkTarget {
                    true_target_id: decoded.clone(),
                    false_target_id: decoded.clone(),
                    generated_target_id: Some(decoded),
                });

            let context_tool = link_data
                .get("context_tool")
                .and_then(|v| v.as_object())
                .map(|tool_data| {
                    let required_fields: Vec<String> = tool_data
                        .get("required_fields")
                        .and_then(|fields| fields.as_array())
                        .map(|fields| {
                            fields
                                .iter()
                                .filter_map(|field| field.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    ContextParse::ParseFields {
                        expected_format: tool_data.clone(),
                        required_fields,
                    }
                });

            let history_tool = link_data
                .get("history_tool")
                .and_then(|v| v.as_object())
                .map(|tool_data| {
                    if let Some(index) = tool_data.get("index").and_then(|v| v.as_u64()) {
                        let field_path: Vec<String> = tool_data
                            .get("field_path")
                            .and_then(|fields| fields.as_array())
                            .map(|fields| {
                                fields
                                    .iter()
                                    .filter_map(|field| field.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();

                        HistoryParse::ExtractField {
                            index: index as usize,
                            field_path,
                        }
                    } else {
                        eprintln!("Invalid or missing 'index' in history_tool configuration.");
                        HistoryParse::CustomProcessor {
                            function: |_| Err("Invalid history_tool configuration".to_string()),
                        }
                    }
                });

            links.insert(
                id.clone(),
                WorkflowLink {
                    id,
                    adapter_id,
                    adapter_type,
                    repetitions,
                    context,
                    target,
                    description,
                    history_tool,
                    context_tool,
                },
            );
        }
    }

    Ok(links)
}
