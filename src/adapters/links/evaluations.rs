use std::{collections::HashMap, error::Error, fmt};
use crate::{
    adapters::nodes::agents::{call_llm_api, Agent, LLMModel},
    nibble::Adaptable,
    utils::generate_unique_id,
};
use ethers::{types::H160, utils::hex};
use reqwest::Client;
use serde_json::{Map, Number, Value};
use tokio::{
    sync::{oneshot, Mutex},
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct Evaluation {
    pub name: String,
    pub encrypted: bool,
    pub id: Vec<u8>,
    pub evaluation_type: EvaluationType,
}

#[derive(Clone)]
pub enum EvaluationType {
    HumanJudge {
        timeout: Duration,
        default: bool,
        endpoint: String,
        auth_key: Option<String>,
    },
    LLMJudge {
        model_type: LLMModel,
        prompt: String,
        approval_threshold: f64,
    },
    AgentJudge {
        agent_id: Vec<u8>,
        prompt: String,
        approval_threshold: f64,
    },
}

#[derive(Debug, Default)]
pub struct HumanJudgeState {
    pub pending_interactions: Mutex<HashMap<String, oneshot::Sender<String>>>,
}

impl HumanJudgeState {
    pub fn new() -> Self {
        Self {
            pending_interactions: Mutex::new(HashMap::new()),
        }
    }
}

impl fmt::Debug for EvaluationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvaluationType::HumanJudge {
                timeout,
                auth_key,
                default,
                endpoint,
            } => f
                .debug_struct("HumanJudge")
                .field("timeout", timeout)
                .field("default", default)
                .field("endpoint", endpoint)
                .field("auth_key", auth_key)
                .finish(),
            EvaluationType::LLMJudge {
                model_type,
                prompt,
                approval_threshold,
            } => f
                .debug_struct("LLMJudge")
                .field("model_type", model_type)
                .field("prompt", prompt)
                .field("approval_threshold", approval_threshold)
                .finish(),
            EvaluationType::AgentJudge {
                agent_id,
                prompt,
                approval_threshold,
            } => f
                .debug_struct("AgentJudge")
                .field("agent_id", agent_id)
                .field("prompt", prompt)
                .field("approval_threshold", approval_threshold)
                .finish(),
        }
    }
}

pub fn configure_new_evaluation(
    name: &str,
    evaluation_type: EvaluationType,
    encrypted: bool,
    address: &H160,
) -> Result<Evaluation, Box<dyn Error + Send + Sync>> {
    let evaluation = Evaluation {
        name: name.to_string(),
        encrypted,
        id: generate_unique_id(address),
        evaluation_type,
    };
    Ok(evaluation)
}

impl Adaptable for Evaluation {
    fn name(&self) -> &str {
        &self.name
    }
    fn id(&self) -> &Vec<u8> {
        &self.id
    }
}

impl EvaluationType {
    pub fn to_json(&self) -> Value {
        match self {
            EvaluationType::HumanJudge {
                timeout,
                auth_key,
                default,
                endpoint,
            } => {
                let mut map = Map::new();
                map.insert("type".to_string(), Value::String("HumanJudge".to_string()));
                map.insert(
                    "timeout".to_string(),
                    Value::Number(Number::from(timeout.as_secs() as i64)),
                );
                map.insert(
                    "auth_key".to_string(),
                    Value::String(auth_key.as_ref().unwrap_or(&"".to_string()).to_string()),
                );
                map.insert("endpoint".to_string(), Value::String(endpoint.to_string()));
                map.insert("default".to_string(), Value::Bool(*default));
                Value::Object(map)
            }
            EvaluationType::LLMJudge {
                model_type,
                prompt,
                approval_threshold,
            } => {
                let mut map = Map::new();
                map.insert("type".to_string(), Value::String("LLMJudge".to_string()));
                map.insert("model_type".to_string(), model_type.to_json());
                map.insert("prompt".to_string(), Value::String(prompt.clone()));
                map.insert(
                    "approval_threshold".to_string(),
                    Value::Number(
                        serde_json::Number::from_f64(*approval_threshold)
                            .expect("Invalid f64 for approval_threshold"),
                    ),
                );
                Value::Object(map)
            }
            EvaluationType::AgentJudge {
                agent_id,
                prompt,
                approval_threshold,
            } => {
                let mut map = Map::new();
                map.insert("type".to_string(), Value::String("AgentJudge".to_string()));
                map.insert(
                    "agent_id".to_string(),
                    Value::Array(agent_id.iter().map(|&b| Value::Number(b.into())).collect()),
                );
                map.insert(
                    "approval_threshold".to_string(),
                    Value::Number(
                        serde_json::Number::from_f64(*approval_threshold)
                            .expect("Invalid f64 for approval_threshold"),
                    ),
                );
                map.insert("prompt".to_string(), Value::String(prompt.to_string()));
                Value::Object(map)
            }
        }
    }
}

impl Evaluation {
    pub fn to_json(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("name".to_string(), Value::String(self.name.clone()));
        map.insert("public".to_string(), Value::Bool(self.encrypted));
        map.insert(
            "evaluation_type".to_string(),
            self.evaluation_type.to_json(),
        );
        map
    }

    pub async fn check_evaluation(
        &self,
        agents: Vec<Agent>,
        flow_previous_context: Option<&str>,
        flow_next_steps: Option<&str>,
        interaction_id: Vec<u8>,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        match &self.evaluation_type {
            EvaluationType::HumanJudge {
                timeout,
                default,
                endpoint,
                auth_key,
            } => {
                let client = Client::new();

                let mut request = client.post(endpoint).json(&serde_json::json!({
                    "interaction_id": hex::encode(&interaction_id),
                    "context": flow_previous_context.unwrap_or("No previous context"),
                    "next_steps": flow_next_steps.unwrap_or("No next steps"),
                }));

                if let Some(key) = auth_key {
                    request = request.header("Authorization", format!("Bearer {}", key));
                }

                match tokio::time::timeout(*timeout, request.send()).await {
                    Ok(Ok(response)) => {
                        if response.status().is_success() {
                            let text = response.text().await?;
                            let decision = text.trim().to_lowercase();
                            match decision.as_str() {
                                "yes" => {
                                    println!("User approved (yes).");
                                    Ok(true)
                                }
                                "no" => {
                                    println!("User rejected (no).");
                                    Ok(false)
                                }
                                _ => {
                                    println!("Invalid response. Using default: {:?}", default);
                                    Ok(*default)
                                }
                            }
                        } else {
                            eprintln!(
                                "Error: Received non-successful status from the user server: {}",
                                response.status()
                            );
                            Ok(*default)
                        }
                    }

                    Ok(Err(err)) => {
                        eprintln!("Error while contacting user server: {:?}", err);
                        Ok(*default)
                    }

                    Err(_) => {
                        println!(
                            "Timeout reached while waiting for user server response. Using default: {:?}",
                            default
                        );
                        Ok(*default)
                    }
                }
            }
            EvaluationType::LLMJudge {
                model_type,
                prompt,
                approval_threshold,
            } => {
                let full_prompt = format!(
                    "{}\n\nContext:\n{}\n\nNext Steps:\n{}\n\nPlease respond in JSON format as follows:\n{{\"score\": <value>}}",
                    prompt,
                    flow_previous_context.unwrap_or("No previous context available."),
                    flow_next_steps.unwrap_or("No next steps available.")
                );

                let llm_response = call_llm_api(model_type, &full_prompt).await?;
                match serde_json::from_str::<serde_json::Value>(&llm_response) {
                    Ok(parsed_response) => {
                        if let Some(score) = parsed_response.get("score").and_then(|v| v.as_f64()) {
                            if score >= *approval_threshold {
                                println!(
                                    "LLM approved the flow continuation with score: {}",
                                    score
                                );
                                Ok(true)
                            } else {
                                println!(
                                    "LLM rejected the flow continuation with score: {} (Threshold: {})",
                                    score, approval_threshold
                                );
                                Ok(false)
                            }
                        } else {
                            eprintln!("Invalid LLM response format. Using default rejection.");
                            Ok(false)
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Error parsing LLM response: {}. Response was: {}",
                            e, llm_response
                        );
                        Ok(false)
                    }
                }
            }
            EvaluationType::AgentJudge {
                agent_id,
                prompt,
                approval_threshold,
            } => {
                if let Some(agent) = agents.iter().find(|agent| agent.id == *agent_id) {
                    let model_type = &agent.model;

                    let full_prompt = format!(
                        "{}\n\nContext:\n{}\n\nNext Steps:\n{}\n\nPlease respond in JSON format as follows:\n{{\"score\": <value>}}",
                        prompt,
                        flow_previous_context.unwrap_or("No previous context available."),
                        flow_next_steps.unwrap_or("No next steps available.")
                    );

                    let llm_response = call_llm_api(model_type, &full_prompt).await?;
                    match serde_json::from_str::<serde_json::Value>(&llm_response) {
                        Ok(parsed_response) => {
                            if let Some(score) =
                                parsed_response.get("score").and_then(|v| v.as_f64())
                            {
                                if score >= *approval_threshold {
                                    println!(
                                        "Agent-based LLM approved the flow continuation with score: {}",
                                        score
                                    );
                                    Ok(true)
                                } else {
                                    println!(
                                        "Agent-based LLM rejected the flow continuation with score: {} (Threshold: {})",
                                        score, approval_threshold
                                    );
                                    Ok(false)
                                }
                            } else {
                                eprintln!("Invalid LLM response format from Agent. Using default rejection.");
                                Ok(false)
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "Error parsing LLM response from Agent: {}. Response was: {}",
                                e, llm_response
                            );
                            Ok(false)
                        }
                    }
                } else {
                    eprintln!(
                        "Agent with ID {:?} not found. Defaulting to rejection.",
                        agent_id
                    );
                    Ok(false)
                }
            }
        }
    }
}
