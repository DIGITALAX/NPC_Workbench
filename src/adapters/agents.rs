use crate::{nibble::Adaptable, utils::generate_unique_id};
use ethers::{core::rand::thread_rng, prelude::*};
use serde_json::{Map, Value};
use std::{error::Error, str::FromStr};

#[derive(Debug, Clone)]
pub enum LLMModel {
    OpenAI {
        api_key: String,
        model: String,
        temperature: f32,
        max_tokens: u32,
        top_p: f32,
        frequency_penalty: f32,
        presence_penalty: f32,
        system_prompt: Option<String>,
    },
    Claude {
        api_key: String,
        model: String,
        temperature: f32,
        max_tokens: u32,
        top_k: Option<u32>,
        top_p: f32,
        system_prompt: Option<String>,
    },
    Ollama {
        api_key: String,
        model: String,
        temperature: f32,
        max_tokens: u32,
        top_p: f32,
        frequency_penalty: f32,
        presence_penalty: f32,
    },
    Other {
        config: std::collections::HashMap<String, String>,
    },
}

#[derive(Debug, Clone)]
pub struct Agent {
    pub name: String,
    pub id: Vec<u8>,
    pub role: String,
    pub personality: String,
    pub system: String,
    pub model: LLMModel,
    pub wallet: LocalWallet,
    pub write_role: bool,
    pub admin_role: bool,
    pub encrypted: bool,
    pub lens_account: Option<String>,
    pub farcaster_account: Option<String>,
}

pub fn configure_new_agent(
    name: &str,
    role: &str,
    personality: &str,
    system: &str,
    write_role: bool,
    admin_role: bool,
    encrypted: bool,
    model: LLMModel,
    address: &H160,
    wallet_address: Option<&H160>,
    lens_account: Option<&str>,
    farcaster_account: Option<&str>,
) -> Result<Agent, Box<dyn Error + Send + Sync>> {
    let mut wallet = LocalWallet::new(&mut thread_rng());

    if let Some(wallet_address) = wallet_address {
        wallet = LocalWallet::from_str(&wallet_address.to_string()).unwrap_or(wallet);
    }

    let agent = Agent {
        name: name.to_string(),
        id: generate_unique_id(address),
        role: role.to_string(),
        personality: personality.to_string(),
        system: system.to_string(),
        model: model.clone(),
        write_role,
        admin_role,
        encrypted,
        wallet: wallet.clone(),
        lens_account: lens_account.map(|s| s.to_string()),
        farcaster_account: farcaster_account.map(|s| s.to_string()),
    };

    Ok(agent)
}

impl Adaptable for Agent {
    fn name(&self) -> &str {
        &self.name
    }
    fn id(&self) -> &Vec<u8> {
        &self.id
    }
}

impl LLMModel {
    pub fn to_json(&self) -> Value {
        match self {
            LLMModel::OpenAI {
                api_key,
                model,
                temperature,
                max_tokens,
                top_p,
                frequency_penalty,
                presence_penalty,
                system_prompt,
            } => {
                let mut map = Map::new();
                map.insert("type".to_string(), Value::String("OpenAI".to_string()));
                map.insert("api_key".to_string(), Value::String(api_key.clone()));
                map.insert("model".to_string(), Value::String(model.clone()));
                map.insert(
                    "temperature".to_string(),
                    Value::String(temperature.to_string()),
                );
                map.insert(
                    "max_tokens".to_string(),
                    Value::Number((*max_tokens).into()),
                );
                map.insert("top_p".to_string(), Value::String(top_p.to_string()));
                map.insert(
                    "frequency_penalty".to_string(),
                    Value::String(frequency_penalty.to_string()),
                );
                map.insert(
                    "presence_penalty".to_string(),
                    Value::String(presence_penalty.to_string()),
                );
                if let Some(prompt) = system_prompt {
                    map.insert("system_prompt".to_string(), Value::String(prompt.clone()));
                }
                Value::Object(map)
            }
            LLMModel::Claude {
                api_key,
                model,
                temperature,
                max_tokens,
                top_k,
                top_p,
                system_prompt,
            } => {
                let mut map = Map::new();
                map.insert("type".to_string(), Value::String("Claude".to_string()));
                map.insert("api_key".to_string(), Value::String(api_key.clone()));
                map.insert("model".to_string(), Value::String(model.clone()));
                map.insert(
                    "temperature".to_string(),
                    Value::String(temperature.to_string()),
                );
                map.insert(
                    "max_tokens".to_string(),
                    Value::Number((*max_tokens).into()),
                );
                if let Some(k) = top_k {
                    map.insert("top_k".to_string(), Value::Number((*k).into()));
                }
                map.insert("top_p".to_string(), Value::String(top_p.to_string()));
                if let Some(prompt) = system_prompt {
                    map.insert("system_prompt".to_string(), Value::String(prompt.clone()));
                }
                Value::Object(map)
            }
            LLMModel::Ollama {
                api_key,
                model,
                temperature,
                max_tokens,
                top_p,
                frequency_penalty,
                presence_penalty,
            } => {
                let mut map = Map::new();
                map.insert("type".to_string(), Value::String("Ollama".to_string()));
                map.insert("api_key".to_string(), Value::String(api_key.clone()));
                map.insert("model".to_string(), Value::String(model.clone()));
                map.insert(
                    "temperature".to_string(),
                    Value::String(temperature.to_string()),
                );
                map.insert(
                    "max_tokens".to_string(),
                    Value::Number((*max_tokens).into()),
                );
                map.insert("top_p".to_string(), Value::String(top_p.to_string()));
                map.insert(
                    "frequency_penalty".to_string(),
                    Value::String(frequency_penalty.to_string()),
                );
                map.insert(
                    "presence_penalty".to_string(),
                    Value::String(presence_penalty.to_string()),
                );
                Value::Object(map)
            }
            LLMModel::Other { config } => {
                let config_map: Map<String, Value> = config
                    .iter()
                    .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                    .collect();
                let mut map = Map::new();
                map.insert("type".to_string(), Value::String("Other".to_string()));
                map.insert("config".to_string(), Value::Object(config_map));
                Value::Object(map)
            }
        }
    }
}

impl Agent {
    pub fn to_json(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("name".to_string(), Value::String(self.name.clone()));
        map.insert("role".to_string(), Value::String(self.role.clone()));
        map.insert(
            "personality".to_string(),
            Value::String(self.personality.clone()),
        );
        map.insert("system".to_string(), Value::String(self.system.clone()));
        map.insert("model".to_string(), self.model.to_json());
        map.insert(
            "wallet_address".to_string(),
            Value::String(format!("{:?}", self.wallet.address())),
        );
        map.insert(
            "lens_account".to_string(),
            Value::String(self.lens_account.clone().unwrap_or_default()),
        );
        map.insert(
            "farcaster_account".to_string(),
            Value::String(self.farcaster_account.clone().unwrap_or_default()),
        );
        map.insert("write_role".to_string(), Value::Bool(self.write_role));
        map.insert("admin_role".to_string(), Value::Bool(self.admin_role));
        map
    }

    pub async fn execute_agent(
        &self,
        input_prompt: &str,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        match &self.model {
            LLMModel::OpenAI {
                api_key,
                model,
                temperature,
                max_tokens,
                top_p,
                frequency_penalty,
                presence_penalty,
                system_prompt,
            } => {
                let prompt = if let Some(system) = system_prompt {
                    format!("{}\n{}", system, input_prompt)
                } else {
                    input_prompt.to_string()
                };

                let client = reqwest::Client::new();
                let response = client
                    .post("https://api.openai.com/v1/completions")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .json(&serde_json::json!({
                        "model": model,
                        "prompt": prompt,
                        "temperature": temperature,
                        "max_tokens": max_tokens,
                        "top_p": top_p,
                        "frequency_penalty": frequency_penalty,
                        "presence_penalty": presence_penalty
                    }))
                    .send()
                    .await?;

                let response_json: Value = response.json().await?;
                let completion = response_json["choices"][0]["text"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                Ok(completion)
            }
            LLMModel::Claude {
                api_key,
                model,
                temperature,
                max_tokens,
                top_k,
                top_p,
                system_prompt,
            } => {
                let prompt = if let Some(system) = system_prompt {
                    format!("{}\n{}", system, input_prompt)
                } else {
                    input_prompt.to_string()
                };

                let client = reqwest::Client::new();
                let response = client
                    .post(format!("https://api.anthropic.com/v1/claude/{model}"))
                    .header("Authorization", format!("Bearer {}", api_key))
                    .json(&serde_json::json!({
                        "prompt": prompt,
                        "temperature": temperature,
                        "max_tokens": max_tokens,
                        "top_k": top_k,
                        "top_p": top_p
                    }))
                    .send()
                    .await?;

                let response_json: Value = response.json().await?;
                let completion = response_json["completion"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                Ok(completion)
            }
            LLMModel::Ollama {
                api_key,
                model,
                temperature,
                max_tokens,
                top_p,
                frequency_penalty,
                presence_penalty,
            } => {
                let client = reqwest::Client::new();
                let response = client
                    .post("https://api.ollama.ai/generate")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .json(&serde_json::json!({
                        "model": model,
                        "prompt": input_prompt,
                        "temperature": temperature,
                        "max_tokens": max_tokens,
                        "top_p": top_p,
                        "frequency_penalty": frequency_penalty,
                        "presence_penalty": presence_penalty
                    }))
                    .send()
                    .await?;

                let response_json: Value = response.json().await?;
                let completion = response_json["text"].as_str().unwrap_or("").to_string();
                Ok(completion)
            }
            LLMModel::Other { config } => {
                return Err("Execution for Other model type is not implemented.".into());
            }
        }
    }
}
