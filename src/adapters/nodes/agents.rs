use crate::{nibble::Adaptable, utils::generate_unique_id};
use ethers::{core::rand::thread_rng, prelude::*};
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::{from_str, json, Map, Value};
use std::{collections, error::Error, iter::Iterator, str::FromStr};

#[derive(Debug, Clone)]
pub enum LLMModel {
    OpenAI {
        api_key: String,
        model: String,
        temperature: f32,
        max_completion_tokens: u32,
        top_p: f32,
        frequency_penalty: f32,
        presence_penalty: f32,
        system_prompt: Option<String>,
        store: Option<bool>,
        metadata: Option<Value>,
        logit_bias: Option<Value>,
        logprobs: Option<bool>,
        top_logprobs: Option<u32>,
        modalities: Option<Vec<String>>,
        stop: Option<Vec<String>>,
        response_format: Option<Value>,
        stream: Option<bool>,
        parallel_tool_calls: Option<bool>,
        user: Option<String>,
    },
    Claude {
        api_key: String,
        model: String,
        temperature: f32,
        max_tokens: u32,
        top_k: Option<u32>,
        top_p: f32,
        system_prompt: Option<String>,
        version: String,
        stop_sequences: Option<Vec<String>>,
        stream: bool,
        metadata: Option<Value>,
        tool_choice: Option<Value>,
        tools: Option<Vec<Value>>,
    },
    Ollama {
        model: String,
        temperature: f32,
        max_tokens: u32,
        top_p: f32,
        frequency_penalty: f32,
        presence_penalty: f32,
        format: Option<String>,
        suffix: Option<String>,
        system: Option<String>,
        template: Option<String>,
        context: Option<Vec<u32>>,
        stream: Option<bool>,
        raw: Option<bool>,
        keep_alive: Option<String>,
        options: Option<serde_json::Value>,
        images: Option<Vec<String>>,
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
                max_completion_tokens,
                top_p,
                frequency_penalty,
                presence_penalty,
                system_prompt,
                store,
                metadata,
                logit_bias,
                logprobs,
                top_logprobs,
                modalities,
                stop,
                response_format,
                stream,
                parallel_tool_calls,
                user,
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
                    "max_completion_tokens".to_string(),
                    Value::Number((*max_completion_tokens).into()),
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
                map.insert("store".to_string(), Value::Bool(store.unwrap_or(false)));
                if let Some(meta) = metadata {
                    map.insert("metadata".to_string(), meta.clone());
                }
                if let Some(logit_bias) = logit_bias {
                    map.insert("logit_bias".to_string(), logit_bias.clone());
                }
                if let Some(logprobs) = logprobs {
                    map.insert("logprobs".to_string(), Value::Bool(*logprobs));
                }
                if let Some(top_logprobs) = top_logprobs {
                    map.insert(
                        "top_logprobs".to_string(),
                        Value::Number((*top_logprobs).into()),
                    );
                }
                if let Some(modalities) = modalities {
                    map.insert(
                        "modalities".to_string(),
                        Value::Array(
                            modalities
                                .iter()
                                .map(|m| Value::String(m.clone()))
                                .collect(),
                        ),
                    );
                }
                if let Some(stop) = stop {
                    map.insert(
                        "stop".to_string(),
                        Value::Array(stop.iter().map(|s| Value::String(s.clone())).collect()),
                    );
                }
                if let Some(response_format) = response_format {
                    map.insert("response_format".to_string(), response_format.clone());
                }
                map.insert("stream".to_string(), Value::Bool(stream.unwrap_or(false)));
                if let Some(parallel_tool_calls) = parallel_tool_calls {
                    map.insert(
                        "parallel_tool_calls".to_string(),
                        Value::Bool(*parallel_tool_calls),
                    );
                }
                if let Some(user) = user {
                    map.insert("user".to_string(), Value::String(user.clone()));
                }

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
                version,
                stop_sequences,
                stream,
                metadata,
                tool_choice,
                tools,
            } => {
                let mut map = Map::new();
                map.insert("type".to_string(), Value::String("Claude".to_string()));
                map.insert("api_key".to_string(), Value::String(api_key.clone()));
                map.insert("version".to_string(), Value::String(version.clone()));
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
                if let Some(sequences) = stop_sequences {
                    map.insert(
                        "stop_sequences".to_string(),
                        Value::Array(sequences.iter().map(|s| Value::String(s.clone())).collect()),
                    );
                }
                map.insert("stream".to_string(), Value::Bool(*stream));
                if let Some(meta) = metadata {
                    map.insert("metadata".to_string(), meta.clone());
                }
                if let Some(tool_choice) = tool_choice {
                    map.insert("tool_choice".to_string(), tool_choice.clone());
                }
                if let Some(tools) = tools {
                    map.insert(
                        "tools".to_string(),
                        Value::Array(tools.iter().map(|t| t.clone()).collect()),
                    );
                }
                Value::Object(map)
            }
            LLMModel::Ollama {
                model,
                temperature,
                max_tokens,
                top_p,
                frequency_penalty,
                presence_penalty,
                format,
                suffix,
                system,
                template,
                context,
                stream,
                raw,
                keep_alive,
                options,
                images,
            } => {
                let mut map = Map::new();
                map.insert("type".to_string(), Value::String("Ollama".to_string()));
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

                if let Some(format) = format {
                    map.insert("format".to_string(), Value::String(format.clone()));
                }
                if let Some(suffix) = suffix {
                    map.insert("suffix".to_string(), Value::String(suffix.clone()));
                }
                if let Some(system) = system {
                    map.insert("system".to_string(), Value::String(system.clone()));
                }
                if let Some(template) = template {
                    map.insert("template".to_string(), Value::String(template.clone()));
                }
                if let Some(context) = context {
                    map.insert(
                        "context".to_string(),
                        Value::Array(
                            context
                                .iter()
                                .map(|&num| Value::Number(serde_json::Number::from(num)))
                                .collect(),
                        ),
                    );
                }
                map.insert("stream".to_string(), Value::Bool(stream.unwrap_or(false)));
                map.insert("raw".to_string(), Value::Bool(raw.unwrap_or(false)));
                if let Some(keep_alive) = keep_alive {
                    map.insert("keep_alive".to_string(), Value::String(keep_alive.clone()));
                }
                if let Some(options) = options {
                    map.insert("options".to_string(), options.clone());
                }
                if let Some(images) = images {
                    map.insert(
                        "images".to_string(),
                        Value::Array(
                            images
                                .iter()
                                .map(|img| Value::String(img.clone()))
                                .collect(),
                        ),
                    );
                }
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

        let re = Regex::new(
            r"(?i)(objective|goal|task|focus|priority):?\s*(?P<description>.+?)\s*(,|;|:|\.)?\s*(priority|rank|importance):?\s*(?P<priority>\d+)",
        )?;
        let mut found_match = false;

        for cap in re.captures_iter(&generated_objective) {
            if let (Some(description), Some(priority_str)) =
                (cap.name("description"), cap.name("priority"))
            {
                let description = description.as_str().trim().to_string();
                let priority: u8 = priority_str.as_str().parse().unwrap_or(1);
                self.add_objective(&description, priority, true);
                found_match = true;
            } else {
                eprintln!("Could not parse objective: {:?}", cap);
            }
        }

        if !found_match {
            eprintln!("Regex did not match. Applying fallback strategy.");
            for line in generated_objective.lines() {
                if let Some(priority_match) = Regex::new(r"(?P<priority>\d+)").unwrap().find(line) {
                    let priority: u8 = priority_match.as_str().parse().unwrap_or(1);
                    let description = line.replace(priority_match.as_str(), "").trim().to_string();
                    if !description.is_empty() {
                        self.add_objective(&description, priority, true);
                    }
                } else {
                    eprintln!("Could not process line: {}", line);
                }
            }
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
            max_completion_tokens,
            top_p,
            frequency_penalty,
            presence_penalty,
            system_prompt,
            store,
            metadata,
            logit_bias,
            logprobs,
            top_logprobs,
            modalities,
            stop,
            response_format,
            stream,
            parallel_tool_calls,
            user,
        } => {
            let mut messages = vec![];

            if let Some(system) = system_prompt {
                messages.push(json!({
                    "role": "system",
                    "content": system
                }));
            }

            messages.push(json!({
                "role": "user",
                "content": input_prompt
            }));

            let client = reqwest::Client::new();
            let mut request_body = json!({
                "model": model,
                "messages": messages,
                "temperature": temperature,
                "max_completion_tokens": max_completion_tokens,
                "top_p": top_p,
                "frequency_penalty": frequency_penalty,
                "presence_penalty": presence_penalty,
                "n": 1,
            });

            if let Some(store) = store {
                request_body["store"] = json!(store);
            }
            if let Some(metadata) = metadata {
                request_body["metadata"] = metadata.clone();
            }
            if let Some(logit_bias) = logit_bias {
                request_body["logit_bias"] = logit_bias.clone();
            }
            if let Some(logprobs) = logprobs {
                request_body["logprobs"] = json!(logprobs);
            }
            if let Some(top_logprobs) = top_logprobs {
                request_body["top_logprobs"] = json!(top_logprobs);
            }
            if let Some(modalities) = modalities {
                request_body["modalities"] = json!(modalities);
            }
            if let Some(stop) = stop {
                request_body["stop"] = json!(stop);
            }
            if let Some(response_format) = response_format {
                request_body["response_format"] = response_format.clone();
            }
            if let Some(stream) = stream {
                request_body["stream"] = json!(stream);
            }
            if let Some(parallel_tool_calls) = parallel_tool_calls {
                request_body["parallel_tool_calls"] = json!(parallel_tool_calls);
            }
            if let Some(user) = user {
                request_body["user"] = json!(user);
            }

            let response = client
                .post("https://api.openai.com/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&request_body)
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
            let completion = response_json["choices"][0]["message"]["content"]
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
            version,
            stop_sequences,
            stream,
            metadata,
            tool_choice,
            tools,
        } => {
            let client = reqwest::Client::new();

            let mut request_body = json!({
                "model": model,
                "messages": vec![json!({
                    "role": "user",
                    "content": input_prompt
                })],
                "temperature": temperature,
                "max_tokens": max_tokens,
                "system": system_prompt,
                "top_k": top_k,
                "top_p": top_p
            });

            if let Some(stop_sequences) = stop_sequences {
                request_body["stop_sequences"] = json!(stop_sequences);
            }

            if let Some(metadata) = metadata {
                request_body["metadata"] = json!(metadata);
            }

            if let Some(tool_choice) = tool_choice {
                request_body["tool_choice"] = json!(tool_choice);
            }

            if let Some(tools) = tools {
                request_body["tools"] = json!(tools);
            }

            request_body["stream"] = json!(stream);

            let response = client
                .post("https://api.anthropic.com/v1/messages")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("anthropic-version", version)
                .json(&request_body)
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
            let completion = response_json["content"]
                .as_array()
                .and_then(|arr| {
                    arr.iter()
                        .find_map(|c| c.get("text").and_then(|t| t.as_str()))
                })
                .unwrap_or("")
                .to_string();

            Ok(completion)
        }
        LLMModel::Ollama {
            model,
            temperature,
            max_tokens,
            top_p,
            frequency_penalty,
            presence_penalty,
            format,
            suffix,
            system,
            template,
            context,
            stream,
            raw,
            keep_alive,
            options,
            images,
        } => {
            let client = reqwest::Client::new();

            let mut request_body = json!({
                "model": model,
                "prompt": input_prompt,
                "temperature": temperature,
                "max_tokens": max_tokens,
                "top_p": top_p,
                "frequency_penalty": frequency_penalty,
                "presence_penalty": presence_penalty,
            });

            if let Some(format) = format {
                request_body["format"] = json!(format);
            }
            if let Some(suffix) = suffix {
                request_body["suffix"] = json!(suffix);
            }
            if let Some(system) = system {
                request_body["system"] = json!(system);
            }
            if let Some(template) = template {
                request_body["template"] = json!(template);
            }
            if let Some(context) = context {
                request_body["context"] = json!(context);
            }
            if let Some(stream) = stream {
                request_body["stream"] = json!(stream);
            }
            if let Some(raw) = raw {
                request_body["raw"] = json!(raw);
            }
            if let Some(keep_alive) = keep_alive {
                request_body["keep_alive"] = json!(keep_alive);
            }
            if let Some(options) = options {
                request_body["options"] = options.clone();
            }
            if let Some(images) = images {
                request_body["images"] = json!(images);
            }

            let response = client
                .post("http://localhost:11434/api/generate")
                .json(&request_body)
                .send()
                .await;

            let response = match response {
                Ok(resp) => resp,
                Err(e) => {
                    eprintln!("Error sending the request to Ollama: {}", e);
                    return Err(e.into());
                }
            };

            if !response.status().is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown Error".to_string());
                return Err(format!("Error en Ollama response: {}", error_text).into());
            }

            let mut completion = String::new();

            let raw_response = response.text().await?;

            for line in raw_response.lines() {
                if line.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<serde_json::Value>(line) {
                    Ok(json) => {
                        if let Some(resp) = json.get("response").and_then(|r| r.as_str()) {
                            completion.push_str(resp);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error processing JSON: {}. Error: {}", line, e);
                    }
                }
            }

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
                request = request.json(&from_str::<Value>(body)?);
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
