use core::fmt;
use std::{collections::HashMap, error::Error};

use reqwest::{Client, Method};
use serde_json::Value;

use crate::nibble::Nibble;

pub struct OffChainConnector {
    pub name: String,
    pub api_url: String,
    pub public: bool,
    pub http_method: Method,
    pub headers: Option<HashMap<String, String>>,
    pub execution_fn: Option<Box<dyn Fn(Value) -> Result<Value, Box<dyn Error>> + Send + Sync>>,
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
    pub fn new(
        name: &str,
        api_url: &str,
        public: bool,
        http_method: Method,
        headers: Option<HashMap<String, String>>,
    ) -> Self {
        OffChainConnector {
            name: name.to_string(),
            api_url: api_url.to_string(),
            public,
            http_method,
            headers,
            execution_fn: None,
        }
    }

    pub fn set_execution_fn<F>(&mut self, f: F)
    where
        F: Fn(Value) -> Result<Value, Box<dyn Error>> + Send + Sync + 'static,
    {
        self.execution_fn = Some(Box::new(f));
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
    execution_fn: Option<Box<dyn Fn(Value) -> Result<Value, Box<dyn Error>> + Send + Sync>>,
) -> Result<(), Box<dyn Error>> {
    nibble.offchain_connectors.push(OffChainConnector {
        name: name.to_string(),
        api_url: api_url.to_string(),
        public,
        http_method,
        headers,
        execution_fn,
    });
    Ok(())
}
