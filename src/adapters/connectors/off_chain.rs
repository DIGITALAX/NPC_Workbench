use core::fmt;
use std::{collections::HashMap, error::Error, sync::Arc};

use reqwest::{Client, Method};
use serde_json::{Map, Value};

use crate::{nibble::{Adaptable, Nibble}, utils::generate_unique_id};

#[derive(Clone)]
pub struct OffChainConnector {
    pub name: String,
    pub id: Vec<u8>,
    pub api_url: String,
    pub public: bool,
    pub http_method: Method,
    pub headers: Option<HashMap<String, String>>,
    pub execution_fn: Option<Arc<dyn Fn(Value) -> Result<Value, Box<dyn Error>> + Send + Sync>>,
}

impl fmt::Debug for OffChainConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OffChainConnector")
            .field("name", &self.name)
            .field("api_url", &self.api_url)
            .field("public", &self.public)
            .field("http_method", &self.http_method)
            .field("headers", &self.headers)
            .field("execution_fn", &"Function pointer")
            .finish()
    }
}

impl OffChainConnector {
    pub fn set_execution_fn<F>(&mut self, f: F)
    where
        F: Fn(Value) -> Result<Value, Box<dyn Error>> + Send + Sync + 'static,
    {
        self.execution_fn = Some(Arc::new(f));
    }

    pub async fn execute_request(&self, payload: Option<Value>) -> Result<Value, Box<dyn Error>> {
        let client = Client::new();
        let mut request = client.request(self.http_method.clone(), &self.api_url);

        if let Some(headers) = &self.headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        if self.http_method == Method::POST {
            if let Some(data) = payload {
                request = request
                    .header("Content-Type", "application/json")
                    .body(data.to_string());
            }
        }

        let response = request.send().await?;
        let response_data: Value = response.json().await?;

        if let Some(exec_fn) = &self.execution_fn {
            return exec_fn(response_data);
        }

        Ok(response_data)
    }
}

pub fn configure_new_offchain_connector(
    nibble: &mut Nibble,
    name: &str,
    api_url: &str,
    public: bool,
    http_method: Method,
    headers: Option<HashMap<String, String>>,
    execution_fn: Option<Arc<dyn Fn(Value) -> Result<Value, Box<dyn Error>> + Send + Sync>>,
) -> Result<OffChainConnector, Box<dyn Error>> {
    let off_chain = OffChainConnector {
        name: name.to_string(),
        id: generate_unique_id(),
        api_url: api_url.to_string(),
        public,
        http_method,
        headers,
        execution_fn,
    };
    nibble.offchain_connectors.push(off_chain.clone());
    Ok(off_chain)
}

impl Adaptable for OffChainConnector {
    fn name(&self) -> &str {
        &self.name
    }
    fn id(&self) -> &Vec<u8> {
        &self.id
    }
}

impl OffChainConnector {
    pub fn to_json(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("name".to_string(), Value::String(self.name.clone()));
        map.insert("api_url".to_string(), Value::String(self.api_url.clone()));
        map.insert("public".to_string(), Value::Bool(self.public));
        map.insert(
            "http_method".to_string(),
            Value::String(self.http_method.as_str().to_string()),
        );

        if let Some(headers) = &self.headers {
            let headers_map: Map<String, Value> = headers
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect();
            map.insert("headers".to_string(), Value::Object(headers_map));
        }

        if self.execution_fn.is_some() {
            map.insert(
                "execution_fn".to_string(),
                Value::String("Function pointer (not serializable)".to_string()),
            );
        }

        map
    }
}
