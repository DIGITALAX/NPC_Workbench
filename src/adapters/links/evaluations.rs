use crate::{
    adapters::nodes::agents::{call_llm_api, Agent, LLMModel},
    nibble::Adaptable,
    utils::generate_unique_id,
};
use ethers::{types::H160, utils::hex};
use reqwest::Client;
use serde_json::{Map, Number, Value};
use std::{collections::HashMap, error::Error, fmt};
use tokio::{
    sync::{oneshot, Mutex},
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct Evaluation {
    pub name: String,
    pub encrypted: bool,
    pub id: String,
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
        response_type: EvaluationResponseType,
    },
    AgentJudge {
        agent_id: String,
        prompt: String,
        response_type: EvaluationResponseType,
    },
}

#[derive(Clone, Debug)]
pub enum EvaluationResponseType {
    Boolean { expected: bool },
    Score { threshold: f64 },
    Dynamic,
}

impl EvaluationResponseType {
    pub fn evaluate(&self, response: &Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        match self {
            EvaluationResponseType::Boolean { expected } => {
                if let Some(value) = response.as_bool() {
                    Ok(Value::Bool(value == *expected))
                } else {
                    Err("Response is not a boolean.".into())
                }
            }
            EvaluationResponseType::Score { threshold } => {
                if let Some(score) = response.get("score").and_then(|v| v.as_f64()) {
                    Ok(Value::Bool(score >= *threshold))
                } else {
                    Err("Response missing 'score' field.".into())
                }
            }
            EvaluationResponseType::Dynamic => Ok(response.clone()),
        }
    }

    pub fn to_json(&self) -> Value {
        match self {
            EvaluationResponseType::Boolean { expected } => Value::Bool(*expected),
            EvaluationResponseType::Score { threshold } => {
                Value::Number(serde_json::Number::from_f64(*threshold).unwrap())
            }
            EvaluationResponseType::Dynamic => Value::String("Dynamic".to_string()),
        }
    }
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
                response_type,
            } => f
                .debug_struct("LLMJudge")
                .field("model_type", model_type)
                .field("prompt", prompt)
                .field("response_type", response_type)
                .finish(),
            EvaluationType::AgentJudge {
                agent_id,
                prompt,
                response_type,
            } => f
                .debug_struct("AgentJudge")
                .field("agent_id", agent_id)
                .field("prompt", prompt)
                .field("response_type", response_type)
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
    fn id(&self) -> &str {
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
                response_type,
            } => {
                let mut map = Map::new();
                map.insert("type".to_string(), Value::String("LLMJudge".to_string()));
                map.insert("model_type".to_string(), model_type.to_json());
                map.insert("prompt".to_string(), Value::String(prompt.clone()));
                map.insert("response_type".to_string(), response_type.to_json());
                Value::Object(map)
            }
            EvaluationType::AgentJudge {
                agent_id,
                prompt,
                response_type,
            } => {
                let mut map = Map::new();
                map.insert("type".to_string(), Value::String("AgentJudge".to_string()));
                map.insert(
                    "agent_id".to_string(),
                    Value::String(agent_id.to_string()),
                );
                map.insert("response_type".to_string(), response_type.to_json());
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
        previous_node_context: Option<Value>,
        flow_previous_context: Option<&str>,
        flow_next_steps: Option<&str>,
        interaction_id: String,
    ) -> Result<Value, Box<dyn Error + Send + Sync>> {
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

                let response = match tokio::time::timeout(*timeout, request.send()).await {
                    Ok(Ok(resp)) if resp.status().is_success() => resp.text().await?,
                    _ => return Ok(Value::Bool(*default)),
                };

                match response.trim().to_lowercase().as_str() {
                    "yes" => Ok(Value::Bool(true)),
                    "no" => Ok(Value::Bool(false)),
                    _ => Ok(Value::Bool(*default)),
                }
            }

            EvaluationType::LLMJudge {
                model_type,
                prompt,
                response_type,
            } => {
                let context_section = if let Some(context) = previous_node_context {
                    format!("{}", context)
                } else {
                    String::new()
                };

                let full_prompt = format!(
                    "{}\n{}\n\nAlso take into consideration the following information when deciding:\n\nContext:\n{}\n\nNext Steps:\n{}",
                    prompt,
                    context_section,
                    flow_previous_context.unwrap_or("No previous context"),
                    flow_next_steps.unwrap_or("No next steps")
                );

                let llm_response = call_llm_api(model_type, &full_prompt).await?;
                let parsed_response: Value = serde_json::from_str(&llm_response)?;

                response_type.evaluate(&parsed_response)
            }
            EvaluationType::AgentJudge {
                agent_id,
                prompt,
                response_type,
            } => {
                if let Some(agent) = agents.iter().find(|a| a.id == *agent_id) {
                    let objectives_summary = agent
                        .objectives
                        .iter()
                        .map(|obj| format!("- {}", obj.description))
                        .collect::<Vec<String>>()
                        .join("\n");

                    let context_section = if let Some(context) = previous_node_context {
                        format!("{}", context)
                    } else {
                        String::new()
                    };

                    let full_prompt = format!(
                        "{}\n{}\n\nAlso take into consideration the following information when deciding:\n\nContext:\n{}\n\nNext Steps:\n{}\n\nAgent Objectives:\n{}",
                        prompt,
                        context_section,
                        flow_previous_context.unwrap_or("No previous context"),
                        flow_next_steps.unwrap_or("No next steps"), objectives_summary
                    );

                    let llm_response = call_llm_api(&agent.model, &full_prompt).await?;
                    let parsed_response: Value = serde_json::from_str(&llm_response)?;

                    response_type.evaluate(&parsed_response)
                } else {
                    Err("Agent not found.".into())
                }
            }
        }
    }
}
