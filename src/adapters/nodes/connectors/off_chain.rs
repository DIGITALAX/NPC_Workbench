use crate::{
    nibble::Adaptable,
    tools::history::HistoryParse,
    utils::generate_unique_id,
    workflow::{SubflowManager, Workflow},
};
use core::fmt;
use ethers::types::H160;
use reqwest::{Client, Method};
use serde_json::{json, Map, Value};
use std::{collections::HashMap, error::Error, io, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub enum ConnectorType {
    REST {
        base_payload: Option<Value>,
    },
    GraphQL {
        query: String,
        variables: Option<HashMap<String, String>>,
    },
}

#[derive(Clone)]
pub struct OffChainConnector {
    pub name: String,
    pub id: String,
    pub connector_type: ConnectorType,
    pub api_url: String,
    pub encrypted: bool,
    pub http_method: Method,
    pub headers: Option<HashMap<String, String>>,
    pub params: Option<HashMap<String, String>>,
    pub auth_tokens: Option<Value>,
    pub auth_subflow: Option<Workflow>,
    pub result_processing_fn:
        Option<Arc<dyn Fn(Value) -> Result<Value, Box<dyn Error + Send + Sync>> + Send + Sync>>,
}

impl fmt::Debug for OffChainConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OffChainConnector")
            .field("name", &self.name)
            .field("id", &self.id)
            .field("connector_type", &self.connector_type)
            .field("api_url", &self.api_url)
            .field("encrypted", &self.encrypted)
            .field("http_method", &self.http_method)
            .field("headers", &self.headers)
            .field("params", &self.params)
            .field("auth_tokens", &self.auth_tokens)
            .field(
                "result_processing_fn",
                &self
                    .result_processing_fn
                    .as_ref()
                    .map(|f| {
                        let ptr = Arc::as_ptr(f) as *const ();
                        format!("Function pointer at: {:p}", ptr)
                    })
                    .unwrap_or_else(|| "None".to_string()),
            )
            .finish()
    }
}
impl OffChainConnector {
    pub async fn execute_offchain_connector(
        &self,
        dynamic_values: Option<Value>,
        subflow_manager: Option<&SubflowManager>,
        history_tool: Option<HistoryParse>,
    ) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let client = Client::new();
        let mut url = self.api_url.clone();
        let mut auth_tokens: Option<Value> = None;

        if let Some(subflow) = &self.auth_subflow {
            if let Some(manager) = subflow_manager {
                let auth_result = manager
                    .execute_subflow(
                        Arc::new(Mutex::new(subflow.clone())),
                        Some(1),
                        false,
                        true,
                        None,
                    )
                    .await;

                match auth_result {
                    Some(Ok(execution_history)) => {
                        println!("Auth subflow executed. History: {:?}", execution_history);

                        if let Some(tool) = &history_tool {
                            match tool.process(execution_history) {
                                Ok(parsed_value) => {
                                    auth_tokens = Some(parsed_value.clone());
                                    println!(
                                        "Auth tokens updated from history tool: {:?}",
                                        auth_tokens
                                    );
                                }
                                Err(e) => {
                                    eprintln!("Error processing history with tool: {}", e);
                                    return Err(Box::new(io::Error::new(io::ErrorKind::Other, e)));
                                }
                            }
                        } else {
                            eprintln!("History tool not provided, auth tokens not updated.");
                            return Err(
                                "History tool is required for auth subflow processing".into()
                            );
                        }
                    }
                    Some(Err(e)) => {
                        eprintln!("Auth subflow execution failed: {}", e);
                        return Err(Box::new(io::Error::new(io::ErrorKind::Other, e)));
                    }
                    None => {
                        eprintln!("Auth subflow returned no history.");
                        return Err("Auth subflow did not return history".into());
                    }
                }
            } else {
                return Err("SubflowManager is required to execute auth subflows".into());
            }
        }

        if let Some(params) = &self.params {
            let query: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            url = format!("{}?{}", url, query.join("&"));
        }

        let mut request = client.request(self.http_method.clone(), &url);

        if let Some(headers) = &self.headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        if let Some(auth_tokens) = auth_tokens {
            if let Some(token) = auth_tokens.get("access_token").and_then(|t| t.as_str()) {
                request = request.header("Authorization", format!("Bearer {}", token));
            }
        }

        if let Some(auth_tokens) = &self.auth_tokens {
            if let Some(token) = auth_tokens.get("access_token").and_then(|t| t.as_str()) {
                request = request.header("Authorization", format!("Bearer {}", token));
            }
        }

        match &self.connector_type {
            ConnectorType::REST { base_payload } => {
                let mut payload = base_payload.clone().unwrap_or(json!({}));
                if let Some(dynamic) = dynamic_values {
                    if let Some(dynamic_map) = dynamic.as_object() {
                        for (key, value) in dynamic_map {
                            payload[key] = value.clone();
                        }
                    }
                }
                request = request
                    .header("Content-Type", "application/json")
                    .body(payload.to_string());
            }
            ConnectorType::GraphQL { query, variables } => {
                let mut merged_variables = variables.clone().unwrap_or_default();

                if let Some(dynamic) = dynamic_values {
                    if let Some(dynamic_map) = dynamic.as_object() {
                        for (key, value) in dynamic_map {
                            merged_variables.insert(key.clone(), value.to_string());
                        }
                    }
                }

                let graphql_payload = json!({
                    "query": query,
                    "variables": merged_variables
                });

                request = request
                    .header("Content-Type", "application/json")
                    .body(graphql_payload.to_string());
            }
        }

        let response = request.send().await?;
        let response_data: Value = response.json().await?;

        if let Some(exec_fn) = &self.result_processing_fn {
            return exec_fn(response_data);
        }

        Ok(response_data)
    }

    pub fn to_json(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("name".to_string(), Value::String(self.name.clone()));
        map.insert("api_url".to_string(), Value::String(self.api_url.clone()));
        map.insert("public".to_string(), Value::Bool(self.encrypted));
        map.insert(
            "http_method".to_string(),
            Value::String(self.http_method.as_str().to_string()),
        );

        match &self.connector_type {
            ConnectorType::REST { base_payload } => {
                map.insert(
                    "connector_type".to_string(),
                    Value::String("REST".to_string()),
                );
                if let Some(payload) = base_payload {
                    map.insert("base_payload".to_string(), payload.clone());
                }
            }
            ConnectorType::GraphQL { query, variables } => {
                map.insert(
                    "connector_type".to_string(),
                    Value::String("GraphQL".to_string()),
                );
                map.insert("query".to_string(), Value::String(query.clone()));
                if let Some(vars) = variables {
                    let vars_json = vars
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
                        .collect::<Map<String, Value>>();
                    map.insert("variables".to_string(), Value::Object(vars_json));
                }
            }
        }

        if let Some(headers) = &self.headers {
            let headers_map: Map<String, Value> = headers
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect();
            map.insert("headers".to_string(), Value::Object(headers_map));
        }

        if let Some(params) = &self.params {
            let params_map: Map<String, Value> = params
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect();
            map.insert("params".to_string(), Value::Object(params_map));
        }

        if let Some(auth_tokens) = &self.auth_tokens {
            map.insert("auth_tokens".to_string(), auth_tokens.clone());
        }

        if self.result_processing_fn.is_some() {
            map.insert(
                "result_processing_fn".to_string(),
                Value::String("Function pointer (not serializable)".to_string()),
            );
        }

        map
    }
}

pub fn configure_new_offchain_connector(
    name: &str,
    connector_type: ConnectorType,
    api_url: &str,
    encrypted: bool,
    http_method: Method,
    headers: Option<HashMap<String, String>>,
    params: Option<HashMap<String, String>>,
    auth_tokens: Option<Value>,
    result_processing_fn: Option<
        Arc<dyn Fn(Value) -> Result<Value, Box<dyn Error + Send + Sync>> + Send + Sync>,
    >,
    address: &H160,
    auth_subflow: Option<Workflow>,
) -> Result<OffChainConnector, Box<dyn Error + Send + Sync>> {
    let off_chain = OffChainConnector {
        name: name.to_string(),
        id: generate_unique_id(address),
        connector_type,
        api_url: api_url.to_string(),
        encrypted,
        http_method,
        headers,
        params,
        auth_tokens,
        result_processing_fn,
        auth_subflow,
    };
    Ok(off_chain)
}

impl Adaptable for OffChainConnector {
    fn name(&self) -> &str {
        &self.name
    }
    fn id(&self) -> &str {
        &self.id
    }
}
