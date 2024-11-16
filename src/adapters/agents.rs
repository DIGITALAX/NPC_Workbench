use crate::{
    nibble::{Adaptable, Nibble},
    utils::generate_unique_id,
};
use ethers::{core::rand::thread_rng, prelude::*};
use serde_json::{Map, Value};
use std::error::Error;

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
}

pub fn configure_new_agent(
    nibble: &mut Nibble,
    name: &str,
    role: &str,
    personality: &str,
    system: &str,
    write_role: bool,
    admin_role: bool,
    encrypted: bool,
    model: LLMModel,
    address: &H160
) -> Result<Agent, Box<dyn Error>> {
    let wallet = LocalWallet::new(&mut thread_rng());

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
    };

    nibble.agents.push(agent.clone());

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
        map.insert("write_role".to_string(), Value::Bool(self.write_role));
        map.insert("admin_role".to_string(), Value::Bool(self.admin_role));
        map
    }
}
