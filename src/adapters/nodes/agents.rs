use crate::{nibble::Adaptable, utils::generate_unique_id};
use ethers::{core::rand::thread_rng, prelude::*};
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::{Map, Value};
use std::{collections, error::Error, iter::Iterator, str::FromStr};

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
        config: collections::HashMap<String, String>,
    },
}

#[derive(Debug, Clone)]
pub struct Objective {
    pub description: String,
    pub priority: u8,
    pub generated: bool,
}

impl TryFrom<&Value> for Objective {
    type Error = String;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let description = value
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing or invalid 'description' field".to_string())?
            .to_string();

        let priority = value
            .get("priority")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| "Missing or invalid 'priority' field".to_string())?
            as u8;

        let generated = value
            .get("generated")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| "Missing or invalid 'generated' field".to_string())?;

        Ok(Objective {
            description,
            priority,
            generated,
        })
    }
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
    pub objectives: Vec<Objective>,
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
    objectives: Vec<Objective>,
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
        objectives,
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
        Ok(call_llm_api(&self.model, input_prompt).await?)
    }

    pub fn add_objective(&mut self, description: &str, priority: u8, generated: bool) {
        let objective = Objective {
            description: description.to_string(),
            priority,
            generated,
        };
        self.objectives.push(objective);
        self.objectives.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub async fn generate_objectives(
        &mut self,
        input_context: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let prompt = format!(
            "As a {} with the personality '{}', what objectives should you focus on given the following context: {}. List each objective on a new line and include a ranking (priority) between 1 and 10, where 10 is the highest priority. Format: Objective: <description>, Priority: <1-10>.",
            self.role, self.personality, input_context
        );

        let generated_objective = self.execute_agent(&prompt).await?;

        let re = Regex::new(r"Objective:\s*(.+),\s*Priority:\s*(\d+)")?;
        for cap in re.captures_iter(&generated_objective) {
            let description = cap[1].trim().to_string();
            let priority: u8 = cap[2].parse().unwrap_or(1);
            self.add_objective(&description, priority, true);
        }

        Ok(())
    }
}

pub async fn call_llm_api(
    model_type: &LLMModel,
    input_prompt: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    match &model_type {
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
                .await;

            let response = match response {
                Ok(resp) => resp,
                Err(e) => {
                    eprintln!("Error sending request to OpenAI API: {}", e);
                    return Err(e.into());
                }
            };

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
                .post(format!("https://api.anthropic.com/v1/claude/{}", model))
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&serde_json::json!({
                    "prompt": prompt,
                    "temperature": temperature,
                    "max_tokens": max_tokens,
                    "top_k": top_k,
                    "top_p": top_p
                }))
                .send()
                .await;

            let response = match response {
                Ok(resp) => resp,
                Err(e) => {
                    eprintln!("Error sending request to Claude API: {}", e);
                    return Err(e.into());
                }
            };

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
                .await;

            let response = match response {
                Ok(resp) => resp,
                Err(e) => {
                    eprintln!("Error sending request to Ollama API: {}", e);
                    return Err(e.into());
                }
            };

            let response_json: Value = response.json().await?;
            let completion = response_json["text"].as_str().unwrap_or("").to_string();
            Ok(completion)
        }
        LLMModel::Other { config } => {
            let url = config.get("url").ok_or("Missing 'url' in configuration.")?;
            let default_method = String::from("POST");
            let method = config.get("method").unwrap_or(&default_method);
            let headers: HeaderMap = config
                .iter()
                .filter(|(key, _)| key.starts_with("header_"))
                .map(|(key, value)| {
                    let header_name =
                        HeaderName::from_bytes(key.trim_start_matches("header_").as_bytes())
                            .map_err(|e| format!("Invalid header name: {}", e))?;
                    let header_value = HeaderValue::from_str(value)
                        .map_err(|e| format!("Invalid header value: {}", e))?;
                    Ok((header_name, header_value))
                })
                .collect::<Result<HeaderMap, String>>()?;

            let client = reqwest::Client::new();

            let mut request = match method.as_str() {
                "GET" => client.get(url),
                "POST" => client.post(url),
                "PUT" => client.put(url),
                "DELETE" => client.delete(url),
                _ => return Err(format!("Unsupported HTTP method: {}", method).into()),
            };

            if !headers.is_empty() {
                request = request.headers(headers);
            }

            if let Some(body) = config.get("body") {
                request = request.json(&serde_json::from_str::<Value>(body)?);
            }

            let response: Result<reqwest::Response, reqwest::Error> = request.send().await;

            let response = match response {
                Ok(resp) => resp,
                Err(e) => {
                    eprintln!("Error sending request to custom API: {}", e);
                    return Err(e.into());
                }
            };

            let response_json: Value = response.json().await?;
            let completion = response_json["result"]
                .as_str()
                .unwrap_or("No result field found in response.")
                .to_string();

            Ok(completion)
        }
    }
}
