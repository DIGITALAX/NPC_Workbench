use crate::{
    encrypt::encrypt_with_public_key,
    ipfs::IPFSClient,
    nibble::{Adapter, Nibble},
    tools::{context::ContextParse, history::HistoryParse},
    utils::generate_unique_id,
};
use chrono::{DateTime, Utc};
use ethers::{
    abi::{Abi, Token, Tokenize},
    middleware::SignerMiddleware,
    prelude::*,
    utils::hex,
};
use serde::Serialize;
use serde_json::{Map, Value};
use std::{
    collections::HashMap, error::Error, fmt::Debug, fs::File, io::Read, marker::Send, path::Path,
    result::Result, str::FromStr, sync::Arc,
};
use tokio::sync::{mpsc, oneshot, Mutex};

#[derive(Debug, Clone)]
pub struct ExecutionHistory {
    pub element_id: Vec<u8>,
    pub element_type: String,
    pub result: Option<Value>,
    pub description: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum NodeAdapter {
    OffChainConnector,
    OnChainConnector,
    Agent,
    SubFlow {
        subflow: Box<Workflow>,
        blocking: bool,
        repetitions: Option<u32>,
        count_successes: bool,
    },
}

#[derive(Debug, Clone)]
pub enum LinkAdapter {
    Condition,
    FHEGate,
    Listener,
    Evaluation,
}

#[derive(Debug, Clone)]
pub struct Workflow {
    pub id: Vec<u8>,
    pub name: String,
    pub nodes: HashMap<Vec<u8>, WorkflowNode>,
    pub links: HashMap<Vec<u8>, WorkflowLink>,
    pub nibble_context: Arc<Nibble>,
    pub encrypted: bool,
    pub execution_history: Vec<ExecutionHistory>,
}

#[derive(Debug, Clone)]
pub struct WorkflowNode {
    pub id: Vec<u8>,
    pub adapter_id: Vec<u8>,
    pub adapter_type: NodeAdapter,
    pub context: Option<Value>,
    pub repetitions: Option<u32>,
    pub description: Option<String>,
    pub context_tool: Option<ContextParse>,
    pub history_tool: Option<HistoryParse>,
}

impl WorkflowNode {
    pub fn to_json(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("id".to_string(), Value::String(hex::encode(&self.id)));
        map.insert(
            "adapter_type".to_string(),
            Value::String(format!("{:?}", self.adapter_type)),
        );
        map.insert(
            "adapter_id".to_string(),
            Value::String(hex::encode(&self.adapter_id)),
        );
        map
    }
}

#[derive(Debug, Clone)]
pub struct LinkTarget {
    pub true_target_id: Vec<u8>,
    pub false_target_id: Vec<u8>,
    pub generated_target_id: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct WorkflowLink {
    pub id: Vec<u8>,
    pub adapter_id: Vec<u8>,
    pub adapter_type: LinkAdapter,
    pub repetitions: Option<u32>,
    pub context: Option<Value>,
    pub target: Option<LinkTarget>,
    pub description: Option<String>,
    pub context_tool: Option<ContextParse>,
    pub history_tool: Option<HistoryParse>,
}

impl WorkflowLink {
    pub fn to_json(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("id".to_string(), Value::String(hex::encode(&self.id)));
        map.insert(
            "adapter_id".to_string(),
            Value::String(hex::encode(&self.adapter_id)),
        );
        map.insert(
            "adapter_type".to_string(),
            Value::String(format!("{:?}", self.adapter_type)),
        );
        map
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ModifyWorkflow {
    pub id: Vec<u8>,
    pub metadata: String,
    pub encrypted: bool,
}

impl Tokenize for ModifyWorkflow {
    fn into_tokens(self) -> Vec<Token> {
        vec![
            Token::Bytes(self.id),
            Token::String(self.metadata),
            Token::Bool(self.encrypted),
        ]
    }
}

impl Workflow {
    pub fn add_node(
        &mut self,
        adapter_id: Vec<u8>,
        adapter_type: NodeAdapter,
        repetitions: Option<u32>,
        context: Option<Value>,
        description: Option<String>,
        context_tool: Option<ContextParse>,
        history_tool: Option<HistoryParse>,
    ) -> &mut Self {
        let id = generate_unique_id(&self.nibble_context.owner_wallet.address());
        self.nodes.insert(
            id.clone(),
            WorkflowNode {
                id,
                adapter_id,
                adapter_type,
                repetitions,
                context,
                description,
                context_tool,
                history_tool,
            },
        );
        self
    }

    pub fn add_link(
        &mut self,
        adapter_id: Vec<u8>,
        adapter_type: LinkAdapter,
        repetitions: Option<u32>,
        context: Option<Value>,
        target: Option<LinkTarget>,
        description: Option<String>,
        context_tool: Option<ContextParse>,
        history_tool: Option<HistoryParse>,
    ) -> &mut Self {
        let id = generate_unique_id(&self.nibble_context.owner_wallet.address());
        self.links.insert(
            id.clone(),
            WorkflowLink {
                id,
                adapter_id,
                adapter_type,
                repetitions,
                context,
                target,
                description,
                context_tool,
                history_tool,
            },
        );
        self
    }

    pub async fn remove(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if self.nibble_context.contracts.len() < 1 {
            return Err("No contracts found. Load or create a Nibble.".into());
        }

        let client = SignerMiddleware::new(
            self.nibble_context.provider.clone(),
            self.nibble_context
                .owner_wallet
                .clone()
                .with_chain_id(self.nibble_context.chain),
        );
        let client = Arc::new(client);

        let storage_contract_address = self
            .nibble_context
            .contracts
            .iter()
            .find(|c| c.name == "NibbleStorage")
            .ok_or("NibbleStorage contract not found")?
            .address;

        let mut abi_file = File::open(Path::new("./abis/NibbleStorage.json"))?;
        let mut abi_content = String::new();
        abi_file.read_to_string(&mut abi_content)?;
        let abi = serde_json::from_str::<Abi>(&abi_content)?;
        let contract_instance = Contract::new(storage_contract_address, abi, client.clone());

        let method = contract_instance.method::<_, H256>("removeWorkflow", self.id.clone());

        match method {
            Ok(call) => {
                let FunctionCall { tx, .. } = call;

                if let Some(tx_request) = tx.as_eip1559_ref() {
                    let gas_price = U256::from(500_000_000_000u64);
                    let max_priority_fee = U256::from(25_000_000_000u64);
                    let gas_limit = U256::from(300_000);

                    let cliente = contract_instance.client().clone();
                    let req = Eip1559TransactionRequest {
                        from: Some(client.address()),
                        to: Some(NameOrAddress::Address(storage_contract_address)),
                        gas: Some(gas_limit),
                        value: tx_request.value,
                        data: tx_request.data.clone(),
                        max_priority_fee_per_gas: Some(max_priority_fee),
                        max_fee_per_gas: Some(gas_price + max_priority_fee),
                        chain_id: Some(Chain::PolygonAmoy.into()),
                        ..Default::default()
                    };

                    let pending_tx = cliente.send_transaction(req, None).await.map_err(|e| {
                        eprintln!("Error sending the transaction: {:?}", e);
                        Box::<dyn Error + Send + Sync>::from(format!(
                            "Error sending the transaction: {}",
                            e
                        ))
                    })?;

                    match pending_tx.await {
                        Ok(Some(receipt)) => receipt,
                        Ok(None) => {
                            return Err("Transaction not recieved".into());
                        }
                        Err(e) => {
                            eprintln!("Error with the transaction: {:?}", e);
                            return Err(e.into());
                        }
                    };
                } else {
                    return Err("EIP-1559 reference invalid.".into());
                }
            }
            Err(e) => {
                eprintln!(
                    "Error while preparing the method of addOrModifyAdaptersBatch: {}",
                    e
                );
                return Err(e.into());
            }
        }

        self.nodes.clear();
        self.links.clear();
        self.id = generate_unique_id(&self.nibble_context.owner_wallet.address());

        Ok(())
    }

    pub async fn persist(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let client = SignerMiddleware::new(
            self.nibble_context.provider.clone(),
            self.nibble_context
                .owner_wallet
                .clone()
                .with_chain_id(self.nibble_context.chain),
        );
        let client = Arc::new(client);

        let storage_contract_address = self
            .nibble_context
            .contracts
            .iter()
            .find(|c| c.name == "NibbleStorage")
            .ok_or("NibbleStorage contract not found")?
            .address;

        let mut abi_file = File::open(Path::new("./abis/NibbleStorage.json"))?;
        let mut abi_content = String::new();
        abi_file.read_to_string(&mut abi_content)?;
        let abi = serde_json::from_str::<Abi>(&abi_content)?;
        let contract_instance = Contract::new(storage_contract_address, abi, client.clone());

        let workflow = self
            .build_workflow(self.nibble_context.ipfs_client.as_ref())
            .await?;

        let method = contract_instance.method::<_, H256>("addOrModifyWorkflow", workflow);

        match method {
            Ok(call) => {
                let FunctionCall { tx, .. } = call;

                if let Some(tx_request) = tx.as_eip1559_ref() {
                    let gas_price = U256::from(500_000_000_000u64);
                    let max_priority_fee = U256::from(25_000_000_000u64);
                    let gas_limit = U256::from(300_000);

                    let cliente = contract_instance.client().clone();
                    let req = Eip1559TransactionRequest {
                        from: Some(client.address()),
                        to: Some(NameOrAddress::Address(storage_contract_address)),
                        gas: Some(gas_limit),
                        value: tx_request.value,
                        data: tx_request.data.clone(),
                        max_priority_fee_per_gas: Some(max_priority_fee),
                        max_fee_per_gas: Some(gas_price + max_priority_fee),
                        chain_id: Some(Chain::PolygonAmoy.into()),
                        ..Default::default()
                    };

                    let pending_tx = cliente.send_transaction(req, None).await.map_err(|e| {
                        eprintln!("Error sending the transaction: {:?}", e);
                        Box::<dyn Error + Send + Sync>::from(format!(
                            "Error sending the transaction: {}",
                            e
                        ))
                    })?;

                    match pending_tx.await {
                        Ok(Some(receipt)) => receipt,
                        Ok(None) => {
                            return Err("Transaction not recieved".into());
                        }
                        Err(e) => {
                            eprintln!("Error with the transaction: {:?}", e);
                            return Err(e.into());
                        }
                    };
                } else {
                    return Err("EIP-1559 reference invalid.".into());
                }
            }
            Err(e) => {
                eprintln!(
                    "Error while preparing the method of addOrModifyAdaptersBatch: {}",
                    e
                );
                return Err(e.into());
            }
        }

        Ok(())
    }

    pub async fn execute(
        &mut self,
        repetitions: Option<u32>,
        count_successes: bool,
    ) -> Result<Vec<ExecutionHistory>, Box<dyn Error>> {
        let mut successful_repeats = 0;
        let mut total_repeats = 0;

        while repetitions.map_or(true, |r| {
            if count_successes {
                successful_repeats < r
            } else {
                total_repeats < r
            }
        }) {
            println!("Executing workflow repetition: {}", total_repeats + 1);
            let mut context_data = None;
            let mut current_success = true;
            let subflow_manager = SubflowManager::new();

            for element_id in self.topological_sort()? {
                if let Some(node) = self.nodes.get(&element_id) {
                    context_data = self
                        .process_node(&node.clone(), Some(&subflow_manager), context_data)
                        .await?;

                    if context_data.is_none() {
                        println!("Execution stopped for repetition: {}", total_repeats + 1);
                        current_success = false;
                        break;
                    }
                } else if let Some(link) = self.links.get(&element_id) {
                    context_data = self
                        .process_link(&link.clone(), context_data, &mut current_success)
                        .await?;

                    if context_data.is_none() {
                        println!("Execution stopped for repetition: {}", total_repeats + 1);
                        break;
                    }
                }
            }

            if current_success && count_successes {
                successful_repeats += 1;
            }

            total_repeats += 1;
        }

        println!(
            "Workflow execution complete. Total: {}, Successful: {}",
            total_repeats, successful_repeats
        );
        Ok(self.execution_history.clone())
    }

    async fn build_workflow(
        &self,
        ipfs_client: &dyn IPFSClient,
    ) -> Result<ModifyWorkflow, Box<dyn Error + Send + Sync>> {
        let mut metadata_map = Map::new();
        metadata_map.insert(
            "links".to_string(),
            Value::Array(
                self.links
                    .iter()
                    .map(|link| Value::Object(link.1.to_json()))
                    .collect(),
            ),
        );
        metadata_map.insert(
            "nodes".to_string(),
            Value::Array(
                self.nodes
                    .iter()
                    .map(|node| Value::Object(node.1.to_json()))
                    .collect(),
            ),
        );
        metadata_map.insert(
            "execution_history".to_string(),
            Value::Array(
                self.execution_history
                    .iter()
                    .map(|entry| {
                        let mut map = Map::new();
                        map.insert(
                            "element_id".to_string(),
                            Value::String(hex::encode(&entry.element_id)),
                        );
                        map.insert(
                            "element_type".to_string(),
                            Value::String(entry.element_type.clone()),
                        );
                        map.insert(
                            "result".to_string(),
                            entry.result.clone().unwrap_or(Value::Null),
                        );
                        map.insert(
                            "timestamp".to_string(),
                            Value::String(entry.timestamp.to_rfc3339()),
                        );
                        Value::Object(map)
                    })
                    .collect(),
            ),
        );

        let mut metadata = serde_json::to_vec(&metadata_map)?;

        if self.encrypted {
            metadata = encrypt_with_public_key(metadata, self.nibble_context.owner_wallet.clone())
                .map_err(|e| -> Box<dyn Error + Send + Sync> {
                    Box::<dyn Error + Send + Sync>::from(e)
                })?;
        }
        let ipfs_hash = ipfs_client.upload(metadata).await?;

        Ok(ModifyWorkflow {
            id: self.id.clone(),
            encrypted: self.encrypted,
            metadata: ipfs_hash,
        })
    }

    fn topological_sort(&self) -> Result<Vec<Vec<u8>>, String> {
        let mut in_degree: HashMap<Vec<u8>, usize> = HashMap::new();
        let mut graph: HashMap<Vec<u8>, Vec<Vec<u8>>> = HashMap::new();

        for node in self.nodes.values() {
            in_degree.insert(node.id.clone(), 0);
            graph.insert(node.id.clone(), Vec::new());
        }

        for link in self.links.values() {
            graph
                .entry(link.adapter_id.clone())
                .or_default()
                .push(link.id.clone());
            *in_degree.entry(link.id.clone()).or_default() += 1;
        }

        let mut stack: Vec<Vec<u8>> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut sorted: Vec<Vec<u8>> = Vec::new();

        while let Some(current) = stack.pop() {
            sorted.push(current.clone());
            if let Some(neighbors) = graph.get(&current) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            stack.push(neighbor.clone());
                        }
                    }
                }
            }
        }

        if sorted.len() != self.nodes.len() + self.links.len() {
            return Err("Cyclic dependency detected in the workflow".to_string());
        }

        Ok(sorted)
    }

    pub fn get_execution_history(&self) -> &Vec<ExecutionHistory> {
        &self.execution_history
    }

    async fn process_node(
        &mut self,
        node: &WorkflowNode,
        subflow_manager: Option<&SubflowManager>,
        context_data: Option<Value>,
    ) -> Result<Option<Value>, Box<dyn Error>> {
        let processed_context = if let Some(context_tool) = &node.context_tool {
            if let Some(data) = context_data {
                match context_tool.process(data) {
                    Ok(parsed_data) => Some(parsed_data),
                    Err(e) => {
                        eprintln!(
                            "Error processing context with ContextTool for node {:?}: {}",
                            node.id, e
                        );
                        None
                    }
                }
            } else {
                eprintln!(
                    "No context data provided to process for node {:?}.",
                    node.id
                );
                None
            }
        } else {
            context_data
        };

        match node.adapter_type.clone() {
            NodeAdapter::Agent => {
                let agent_found = self
                    .nibble_context
                    .agents
                    .iter()
                    .find(|agent| agent.id == *node.adapter_id);
                if let Some(agent) = agent_found {
                    println!("Executing Agent: {:?}", node.id);

                    let input_context = node
                        .context
                        .as_ref()
                        .map_or("", |v| v.as_str().unwrap_or(""));

                    match agent.execute_agent(input_context).await {
                        Ok(result) => {
                            println!("Agent Result: {}", result);

                            self.execution_history.push(ExecutionHistory {
                                element_id: node.id.clone(),
                                element_type: Adapter::Agent.to_string(),
                                result: Some(Value::String(result.clone())),
                                timestamp: chrono::Utc::now(),
                                description: None,
                            });
                            Ok(Some(Value::String(result)))
                        }
                        Err(e) => {
                            eprintln!("Agent execution failed: {:?}", e);
                            self.execution_history.push(ExecutionHistory {
                                element_id: node.id.clone(),
                                element_type: Adapter::Agent.to_string(),
                                result: None,
                                timestamp: chrono::Utc::now(),
                                description: None,
                            });
                            Ok(None)
                        }
                    }
                } else {
                    eprintln!("Agent not found for ID: {:?}", node.adapter_id);
                    self.execution_history.push(ExecutionHistory {
                        element_id: node.id.clone(),
                        element_type: Adapter::Agent.to_string(),
                        result: None,
                        timestamp: chrono::Utc::now(),
                        description: None,
                    });
                    Ok(None)
                }
            }
            NodeAdapter::OnChainConnector => {
                let connector_found = self
                    .nibble_context
                    .onchain_connectors
                    .iter()
                    .find(|connector| connector.id == *node.adapter_id);

                if let Some(onchain_connector) = connector_found {
                    println!("Executing OnChainConnector: {:?}", node.id);

                    let (wallet, method_name, params) = if let Some(context) = &node.context {
                        let wallet = if let Some(wallet_name) = context.get("agent_wallet") {
                            if let Some(agent_id) = wallet_name.as_str() {
                                let agent_wallet = self.nodes.values().find_map(|node| {
                                    if let NodeAdapter::Agent = node.adapter_type {
                                        if node.adapter_id == agent_id.as_bytes().to_vec() {
                                            Some(
                                                self.nibble_context
                                                    .agents
                                                    .iter()
                                                    .find(|agent| {
                                                        agent.id == agent_id.as_bytes().to_vec()
                                                    })?
                                                    .wallet
                                                    .clone(),
                                            )
                                        } else {
                                            self.execution_history.push(ExecutionHistory {
                                                element_id: node.id.clone(),
                                                element_type: Adapter::OnChainConnector.to_string(),
                                                result: None,
                                                timestamp: chrono::Utc::now(),
                                                description: None,
                                            });
                                            None
                                        }
                                    } else {
                                        self.execution_history.push(ExecutionHistory {
                                            element_id: node.id.clone(),
                                            element_type: Adapter::OnChainConnector.to_string(),
                                            result: None,
                                            timestamp: chrono::Utc::now(),
                                            description: None,
                                        });
                                        None
                                    }
                                });

                                if let Some(wallet) = agent_wallet {
                                    wallet
                                } else {
                                    eprintln!(
                                        "Agent with ID {:?} not found, using owner_wallet",
                                        agent_id
                                    );
                                    self.nibble_context.owner_wallet.clone()
                                }
                            } else {
                                eprintln!("Invalid agent_wallet context, using owner_wallet");
                                self.nibble_context.owner_wallet.clone()
                            }
                        } else if let Some(custom_wallet) = context.get("custom_wallet") {
                            if let Some(wallet_str) = custom_wallet.as_str() {
                                let parsed_wallet = Wallet::from_str(wallet_str).map_err(|e| {
                                    eprintln!("Failed to parse custom wallet: {:?}", e);
                                    e
                                });
                                match parsed_wallet {
                                    Ok(wallet) => wallet,
                                    Err(_) => {
                                        eprintln!("Invalid custom wallet, using owner_wallet");
                                        self.nibble_context.owner_wallet.clone()
                                    }
                                }
                            } else {
                                eprintln!("Invalid custom_wallet context, using owner_wallet");
                                self.nibble_context.owner_wallet.clone()
                            }
                        } else {
                            self.nibble_context.owner_wallet.clone()
                        };

                        let method_name = context.get("method_name").and_then(|v| v.as_str());

                        let params = context
                            .get("params")
                            .and_then(|v| v.as_array().map(|arr| arr.clone()));

                        (wallet, method_name, params)
                    } else {
                        (self.nibble_context.owner_wallet.clone(), None, None)
                    };

                    match onchain_connector
                        .execute_onchain_connector(
                            self.nibble_context.provider.clone(),
                            wallet,
                            method_name,
                            params,
                        )
                        .await
                    {
                        Ok(result) => {
                            println!("OnChainConnector executed successfully: {:?}", node.id);
                            let receipt_value = serde_json::to_value(&result).map_err(|e| {
                                println!("Failed to serialize TransactionReceipt: {:?}", e);
                                e
                            })?;

                            self.execution_history.push(ExecutionHistory {
                                element_id: node.id.clone(),
                                element_type: Adapter::OnChainConnector.to_string(),
                                result: Some(receipt_value.clone()),
                                timestamp: chrono::Utc::now(),
                                description: None,
                            });
                            Ok(Some(receipt_value))
                        }
                        Err(e) => {
                            eprintln!("OnChainConnector execution failed: {:?}", e);
                            self.execution_history.push(ExecutionHistory {
                                element_id: node.id.clone(),
                                element_type: Adapter::OnChainConnector.to_string(),
                                result: None,
                                timestamp: chrono::Utc::now(),
                                description: None,
                            });
                            Ok(None)
                        }
                    }
                } else {
                    eprintln!("OnChainConnector not found for ID: {:?}", node.adapter_id);
                    self.execution_history.push(ExecutionHistory {
                        element_id: node.id.clone(),
                        element_type: Adapter::OnChainConnector.to_string(),
                        result: None,
                        timestamp: chrono::Utc::now(),
                        description: None,
                    });
                    Ok(None)
                }
            }

            NodeAdapter::OffChainConnector => {
                let connector_found = self
                    .nibble_context
                    .offchain_connectors
                    .iter()
                    .find(|connector| connector.id == *node.adapter_id);

                if let Some(offchain_connector) = connector_found {
                    println!("Executing OffChainConnector: {:?}", node.id);

                    match offchain_connector
                        .execute_offchain_connector(
                            processed_context.clone(),
                            subflow_manager,
                            node.history_tool.clone(),
                        )
                        .await
                    {
                        Ok(response) => {
                            println!("OffChainConnector response: {:?}", response);
                            self.execution_history.push(ExecutionHistory {
                                element_id: node.id.clone(),
                                element_type: Adapter::OffChainConnector.to_string(),
                                result: Some(response.clone()),
                                timestamp: chrono::Utc::now(),
                                description: None,
                            });
                            Ok(Some(response))
                        }
                        Err(e) => {
                            eprintln!("OffChainConnector execution failed: {:?}", e);
                            self.execution_history.push(ExecutionHistory {
                                element_id: node.id.clone(),
                                element_type: Adapter::OffChainConnector.to_string(),
                                result: None,
                                timestamp: chrono::Utc::now(),
                                description: None,
                            });
                            Ok(None)
                        }
                    }
                } else {
                    eprintln!("OffChainConnector not found for ID: {:?}", node.adapter_id);
                    self.execution_history.push(ExecutionHistory {
                        element_id: node.id.clone(),
                        element_type: Adapter::OffChainConnector.to_string(),
                        result: None,
                        timestamp: chrono::Utc::now(),
                        description: None,
                    });
                    Ok(None)
                }
            }

            NodeAdapter::SubFlow {
                subflow,
                blocking,
                repetitions,
                count_successes,
            } => {
                println!("Executing SubFlow: {:?}", subflow.id);

                if blocking {
                    let result = match subflow_manager {
                        Some(manager) => {
                            let result = manager
                                .execute_subflow(
                                    Arc::new(Mutex::new(*subflow.clone())),
                                    repetitions,
                                    count_successes,
                                    true,
                                    None,
                                )
                                .await;

                            result
                        }
                        None => None,
                    };

                    match result {
                        Some(Ok(history)) => {
                            self.execution_history.push(ExecutionHistory {
                                element_id: node.id.clone(),
                                element_type: "Subflow".to_string(),
                                result: Some(Value::String("Blocking SubFlow Success".to_string())),
                                timestamp: chrono::Utc::now(),
                                description: None,
                            });
                            self.execution_history.extend(history);
                            Ok(Some(Value::String("Blocking SubFlow Success".to_string())))
                        }
                        Some(Err(e)) => {
                            eprintln!("Blocking SubFlow failed: {:?}", e);
                            self.execution_history.push(ExecutionHistory {
                                element_id: node.id.clone(),
                                element_type: "Subflow".to_string(),
                                result: None,
                                timestamp: chrono::Utc::now(),
                                description: None,
                            });
                            Ok(None)
                        }
                        None => {
                            self.execution_history.push(ExecutionHistory {
                                element_id: node.id.clone(),
                                element_type: "Subflow".to_string(),
                                result: None,
                                timestamp: chrono::Utc::now(),
                                description: None,
                            });
                            Ok(None)
                        }
                    }
                } else {
                    match subflow_manager {
                        Some(manager) => {
                            let (report_sender, mut report_receiver) = mpsc::channel(100);

                            manager
                                .execute_subflow(
                                    Arc::new(Mutex::new(*subflow.clone())),
                                    repetitions,
                                    count_successes,
                                    false,
                                    Some(report_sender),
                                )
                                .await;

                            let (tx, rx) = tokio::sync::oneshot::channel();
                            tokio::spawn(async move {
                                while let Some(history) = report_receiver.recv().await {
                                    let _ = tx.send(history);
                                    break;
                                }
                            });

                            if let Ok(history) = rx.await {
                                self.execution_history.push(ExecutionHistory {
                                    element_id: node.id.clone(),
                                    element_type: "Subflow".to_string(),
                                    result: Some(Value::String(
                                        "Blocking SubFlow Success".to_string(),
                                    )),
                                    timestamp: chrono::Utc::now(),
                                    description: None,
                                });
                                self.execution_history.extend(history);
                            } else {
                                self.execution_history.push(ExecutionHistory {
                                    element_id: node.id.clone(),
                                    element_type: "Subflow".to_string(),
                                    result: None,
                                    timestamp: chrono::Utc::now(),
                                    description: None,
                                });
                                eprintln!("Failed to receive history from non-blocking SubFlow.");
                                return Ok(None);
                            }
                        }
                        None => {
                            self.execution_history.push(ExecutionHistory {
                                element_id: node.id.clone(),
                                element_type: "Subflow".to_string(),
                                result: None,
                                timestamp: chrono::Utc::now(),
                                description: None,
                            });
                            eprintln!("No SubflowManager available.");
                            return Ok(None);
                        }
                    }
                    Ok(Some(Value::String(
                        "Non-blocking SubFlow Started".to_string(),
                    )))
                }
            }
        }
    }

    async fn process_link(
        &mut self,
        link: &WorkflowLink,
        context_data: Option<Value>,
        current_success: &mut bool,
    ) -> Result<Option<Value>, Box<dyn Error>> {
        let processed_context = if let Some(context_tool) = &link.context_tool {
            if let Some(data) = context_data {
                match context_tool.process(data) {
                    Ok(parsed_data) => Some(parsed_data),
                    Err(e) => {
                        eprintln!(
                            "Error processing context with ContextTool for node {:?}: {}",
                            link.id, e
                        );
                        None
                    }
                }
            } else {
                eprintln!(
                    "No context data provided to process for node {:?}.",
                    link.id
                );
                None
            }
        } else {
            context_data
        };

        match link.adapter_type {
            LinkAdapter::Condition => {
                println!("Processing Condition: {:?}", link.id);

                let condition_found = self
                    .nibble_context
                    .conditions
                    .iter()
                    .find(|condition| condition.id == *link.adapter_id);

                if let Some(condition) = condition_found {
                    match condition
                        .check_condition(
                            &self.nibble_context,
                            processed_context.clone(),
                            link.context.clone(),
                        )
                        .await
                    {
                        Ok(response) => {
                            println!("Condition response: {:?}", response);

                            self.execution_history.push(ExecutionHistory {
                                element_id: link.id.clone(),
                                element_type: Adapter::Condition.to_string(),
                                result: Some(Value::String("Condition Success".to_string())),
                                timestamp: chrono::Utc::now(),
                                description: None,
                            });

                            if let Some(target) = &link.target {
                                let next_node_id = if response {
                                    &target.true_target_id
                                } else {
                                    &target.false_target_id
                                };

                                if let Some(node) = self.nodes.get(next_node_id) {
                                    println!(
                                        "Continuing to node based on condition: {:?}",
                                        next_node_id
                                    );
                                    let result = self
                                        .process_node(
                                            &node.clone(),
                                            None,
                                            processed_context.clone(),
                                        )
                                        .await?;
                                    self.execution_history.push(ExecutionHistory {
                                        element_id: link.id.clone(),
                                        element_type: Adapter::Condition.to_string(),
                                        result: result.clone(),
                                        timestamp: chrono::Utc::now(),
                                        description: None,
                                    });

                                    Ok(result)
                                } else {
                                    eprintln!(
                                        "Target node not found for condition response: {:?}",
                                        next_node_id
                                    );
                                    self.execution_history.push(ExecutionHistory {
                                        element_id: link.id.clone(),
                                        element_type: Adapter::Condition.to_string(),
                                        result: None,
                                        timestamp: chrono::Utc::now(),
                                        description: None,
                                    });
                                    Ok(None)
                                }
                            } else {
                                if response {
                                    println!("Condition passed, continuing flow.");
                                    self.execution_history.push(ExecutionHistory {
                                        element_id: link.id.clone(),
                                        element_type: Adapter::Condition.to_string(),
                                        result: Some(Value::String(
                                            "Condition Success".to_string(),
                                        )),
                                        timestamp: chrono::Utc::now(),
                                        description: None,
                                    });
                                    Ok(Some(Value::String("Condition Success".to_string())))
                                } else {
                                    println!("Condition failed, stopping flow.");
                                    self.execution_history.push(ExecutionHistory {
                                        element_id: link.id.clone(),
                                        element_type: Adapter::Condition.to_string(),
                                        result: None,
                                        timestamp: chrono::Utc::now(),
                                        description: None,
                                    });
                                    Ok(None)
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Condition execution failed: {:?}", e);

                            self.execution_history.push(ExecutionHistory {
                                element_id: link.id.clone(),
                                element_type: Adapter::Condition.to_string(),
                                result: None,
                                timestamp: chrono::Utc::now(),
                                description: None,
                            });
                            Ok(None)
                        }
                    }
                } else {
                    eprintln!("Condition not found for ID: {:?}", link.adapter_id);

                    self.execution_history.push(ExecutionHistory {
                        element_id: link.id.clone(),
                        element_type: Adapter::Condition.to_string(),
                        result: None,
                        timestamp: chrono::Utc::now(),
                        description: None,
                    });
                    Ok(None)
                }
            }
            LinkAdapter::Listener => {
                println!("Waiting on Listener: {:?}", link.id);

                let listener_found = self
                    .nibble_context
                    .listeners
                    .iter()
                    .find(|listener| listener.id == *link.adapter_id);

                if let Some(listener) = listener_found {
                    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

                    let repetitions = link
                        .context
                        .as_ref()
                        .and_then(|v| v.as_number().and_then(|n| n.as_u64()));

                    let listener_task = tokio::spawn({
                        let listener = listener.clone();
                        async move {
                            if let Err(e) = listener.listen_and_trigger(tx, repetitions).await {
                                eprintln!("Error in listener: {:?}", e);
                            }
                        }
                    });

                    let result = match rx.recv().await {
                        Some(event_data) => {
                            println!("Listener triggered with data: {:?}", event_data);
                            self.execution_history.push(ExecutionHistory {
                                element_id: link.id.clone(),
                                element_type: Adapter::Listener.to_string(),
                                result: Some(event_data.clone()),
                                timestamp: chrono::Utc::now(),
                                description: None,
                            });
                            Some(event_data)
                        }
                        None => {
                            eprintln!("Listener did not produce any result.");
                            *current_success = false;
                            self.execution_history.push(ExecutionHistory {
                                element_id: link.id.clone(),
                                element_type: Adapter::Listener.to_string(),
                                result: None,
                                timestamp: chrono::Utc::now(),
                                description: None,
                            });
                            None
                        }
                    };

                    if let Err(e) = listener_task.await {
                        eprintln!("Listener task failed: {:?}", e);
                        self.execution_history.push(ExecutionHistory {
                            element_id: link.id.clone(),
                            element_type: Adapter::Listener.to_string(),
                            result: None,
                            timestamp: chrono::Utc::now(),
                            description: None,
                        });
                    }

                    Ok(result)
                } else {
                    eprintln!("Listener not found for ID: {:?}", link.adapter_id);
                    *current_success = false;
                    self.execution_history.push(ExecutionHistory {
                        element_id: link.id.clone(),
                        element_type: Adapter::Listener.to_string(),
                        result: None,
                        timestamp: chrono::Utc::now(),
                        description: None,
                    });
                    Ok(None)
                }
            }
            LinkAdapter::FHEGate => {
                println!("Processing FHEGate: {:?}", link.id);
                let fhe_gate_found = self
                    .nibble_context
                    .fhe_gates
                    .iter()
                    .find(|fhe_gate| fhe_gate.id == *link.adapter_id);

                if let Some(fhe_gate) = fhe_gate_found {
                    let encrypted_value_option = if let Some(v) = processed_context.clone() {
                        if let Some(s) = v.as_str() {
                            hex::decode(s).ok()
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    match encrypted_value_option {
                        Some(encrypted_value) => {
                            if let Some(context) = &link.context {
                                match fhe_gate
                                    .check_fhe_gate(
                                        encrypted_value,
                                        context
                                            .get("criterion")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.as_bytes().to_vec()),
                                        context
                                            .get("client_key")
                                            .and_then(|v| serde_json::from_value(v.clone()).ok())
                                            .unwrap(),
                                        self.nibble_context.provider.clone(),
                                        self.nibble_context.owner_wallet.clone(),
                                    )
                                    .await
                                {
                                    Ok(response) => {
                                        if let Some(target) = &link.target {
                                            let next_node_id = if response {
                                                &target.true_target_id
                                            } else {
                                                &target.false_target_id
                                            };

                                            if let Some(node) = self.nodes.get(next_node_id) {
                                                println!(
                                                    "Continuing to node based on FHE gate: {:?}",
                                                    next_node_id
                                                );
                                                let result = self
                                                    .process_node(
                                                        &node.clone(),
                                                        None,
                                                        processed_context.clone(),
                                                    )
                                                    .await?;
                                                self.execution_history.push(ExecutionHistory {
                                                    element_id: link.id.clone(),
                                                    element_type: Adapter::FHEGate.to_string(),
                                                    result: result.clone(),
                                                    timestamp: chrono::Utc::now(),
                                                    description: None,
                                                });

                                                Ok(result)
                                            } else {
                                                eprintln!(
                                    "Target node not found for FHE gate response: {:?}",
                                    next_node_id
                                );

                                                self.execution_history.push(ExecutionHistory {
                                                    element_id: link.id.clone(),
                                                    element_type: Adapter::FHEGate.to_string(),
                                                    result: None,
                                                    timestamp: chrono::Utc::now(),
                                                    description: None,
                                                });
                                                Ok(None)
                                            }
                                        } else {
                                            if response {
                                                println!("FHE gate passed, continuing flow.");

                                                self.execution_history.push(ExecutionHistory {
                                                    element_id: link.id.clone(),
                                                    element_type: Adapter::FHEGate.to_string(),
                                                    result: Some(Value::String(
                                                        "FHE Gate Success".to_string(),
                                                    )),
                                                    timestamp: chrono::Utc::now(),
                                                    description: None,
                                                });
                                                Ok(Some(Value::String(
                                                    "FHE Gate Success".to_string(),
                                                )))
                                            } else {
                                                println!("FHE gate failed, stopping flow.");

                                                self.execution_history.push(ExecutionHistory {
                                                    element_id: link.id.clone(),
                                                    element_type: Adapter::FHEGate.to_string(),
                                                    result: None,
                                                    timestamp: chrono::Utc::now(),
                                                    description: None,
                                                });
                                                Ok(None)
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("FHEGate execution failed: {:?}", e);

                                        self.execution_history.push(ExecutionHistory {
                                            element_id: link.id.clone(),
                                            element_type: Adapter::FHEGate.to_string(),
                                            result: None,
                                            timestamp: chrono::Utc::now(),
                                            description: None,
                                        });
                                        Ok(None)
                                    }
                                }
                            } else {
                                self.execution_history.push(ExecutionHistory {
                                    element_id: link.id.clone(),
                                    element_type: Adapter::FHEGate.to_string(),
                                    result: None,
                                    timestamp: chrono::Utc::now(),
                                    description: None,
                                });
                                Ok(None)
                            }
                        }

                        None => {
                            eprintln!(
                                "Encrypted value from previous node not found for ID: {:?}",
                                link.adapter_id
                            );

                            self.execution_history.push(ExecutionHistory {
                                element_id: link.id.clone(),
                                element_type: Adapter::FHEGate.to_string(),
                                result: None,
                                timestamp: chrono::Utc::now(),
                                description: None,
                            });
                            Ok(None)
                        }
                    }
                } else {
                    eprintln!("FHEGate not found for ID: {:?}", link.adapter_id);
                    self.execution_history.push(ExecutionHistory {
                        element_id: link.id.clone(),
                        element_type: Adapter::FHEGate.to_string(),
                        result: None,
                        timestamp: chrono::Utc::now(),
                        description: None,
                    });
                    Ok(None)
                }
            }
            LinkAdapter::Evaluation => {
                println!("Processing Evaluation: {:?}", link.id);
                let evaluation_found = self
                    .nibble_context
                    .evaluations
                    .iter()
                    .find(|evaluation| evaluation.id == *link.adapter_id);

                if let Some(evaluation) = evaluation_found {
                    let interaction_id = link
                        .context
                        .as_ref()
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_u64().map(|n| n as u8))
                                .collect::<Vec<u8>>()
                        })
                        .unwrap_or_else(Vec::new);

                    let flow_previous_context = self
                        .execution_history
                        .iter()
                        .map(|entry| {
                            format!(
                                "Element ID: {}, Type: {}, Result: {:?}, Timestamp: {}",
                                hex::encode(&entry.element_id),
                                entry.element_type,
                                entry.result.clone().unwrap_or(Value::Null),
                                entry.timestamp
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    let executed_ids: Vec<Vec<u8>> = self
                        .execution_history
                        .iter()
                        .map(|entry| entry.element_id.clone())
                        .collect();

                    let flow_next_steps = self
                        .topological_sort()?
                        .iter()
                        .filter(|id| !executed_ids.contains(id))
                        .map(|id| {
                            if let Some(node) = self.nodes.get(id) {
                                format!(
                                    "Node ID: {}, Adapter Type: {:?}, Description: {:?}",
                                    hex::encode(&node.id),
                                    node.adapter_type,
                                    node.description
                                )
                            } else if let Some(link) = self.links.get(id) {
                                format!(
                                    "Link ID: {}, Adapter Type: {:?}",
                                    hex::encode(&link.id),
                                    link.adapter_type
                                )
                            } else {
                                format!("Unknown element ID: {}", hex::encode(id))
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    match evaluation
                        .check_evaluation(
                            self.nibble_context.agents.clone(),
                            processed_context.clone(),
                            Some(&flow_previous_context),
                            Some(&flow_next_steps),
                            interaction_id,
                        )
                        .await
                    {
                        Ok(response) => {
                            if let Some(target) = &link.target {
                                let mut next_node_id: &Vec<u8> = &Vec::new();

                                if let Some(response_value) = response.as_bool() {
                                    next_node_id = if response_value {
                                        &target.true_target_id
                                    } else {
                                        &target.false_target_id
                                    };
                                } else {
                                    if let Some(generated_target) = &target.generated_target_id {
                                        next_node_id = &generated_target;
                                    }
                                }

                                if let Some(node) = self.nodes.get(next_node_id) {
                                    println!(
                                        "Continuing to node based on Evaluation: {:?}",
                                        next_node_id
                                    );
                                    let result = self
                                        .process_node(
                                            &node.clone(),
                                            None,
                                            processed_context.clone(),
                                        )
                                        .await?;

                                    self.execution_history.push(ExecutionHistory {
                                        element_id: link.id.clone(),
                                        element_type: Adapter::Evaluation.to_string(),
                                        result: result.clone(),
                                        timestamp: chrono::Utc::now(),
                                        description: None,
                                    });

                                    Ok(result)
                                } else {
                                    eprintln!(
                                        "Target node not found for Evaluation response: {:?}",
                                        next_node_id
                                    );
                                    self.execution_history.push(ExecutionHistory {
                                        element_id: link.id.clone(),
                                        element_type: Adapter::Evaluation.to_string(),
                                        result: None,
                                        timestamp: chrono::Utc::now(),
                                        description: None,
                                    });
                                    Ok(None)
                                }
                            } else {
                                println!("Evaluation passed, continuing flow.");
                                self.execution_history.push(ExecutionHistory {
                                    element_id: link.id.clone(),
                                    element_type: Adapter::Evaluation.to_string(),
                                    result: processed_context.clone(),
                                    timestamp: chrono::Utc::now(),
                                    description: None,
                                });

                                Ok(processed_context)
                            }
                        }
                        Err(e) => {
                            eprintln!("Evaluation execution failed: {:?}", e);
                            self.execution_history.push(ExecutionHistory {
                                element_id: link.id.clone(),
                                element_type: Adapter::Evaluation.to_string(),
                                result: None,
                                timestamp: chrono::Utc::now(),
                                description: None,
                            });
                            Ok(None)
                        }
                    }
                } else {
                    eprintln!("Evaluation not found for ID: {:?}", link.adapter_id);
                    self.execution_history.push(ExecutionHistory {
                        element_id: link.id.clone(),
                        element_type: Adapter::Evaluation.to_string(),
                        result: None,
                        timestamp: chrono::Utc::now(),
                        description: None,
                    });
                    Ok(None)
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct SubflowManager {
    sender: mpsc::Sender<SubflowRequest>,
}

#[derive(Debug)]
pub struct SubflowRequest {
    subflow: Arc<Mutex<Workflow>>,
    repetitions: Option<u32>,
    count_successes: bool,
    blocking: bool,
    responder: Option<oneshot::Sender<Result<Vec<ExecutionHistory>, String>>>,
    report_sender: Option<mpsc::Sender<Vec<ExecutionHistory>>>,
}

impl SubflowManager {
    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::channel(100);

        tokio::spawn(async move {
            while let Some(request) = receiver.recv().await {
                let SubflowRequest {
                    subflow,
                    repetitions,
                    count_successes,
                    blocking,
                    responder,
                    report_sender,
                } = request;

                if blocking {
                    let result = {
                        let mut subflow = subflow.lock().await;
                        subflow
                            .execute(repetitions, count_successes)
                            .await
                            .map_err(|e| e.to_string())
                    };
                    if let Some(responder) = responder {
                        let _ = responder.send(result);
                    }
                } else {
                    let subflow_clone = subflow.clone();
                    tokio::spawn(async move {
                        let result = {
                            let mut subflow = subflow_clone.lock().await;
                            subflow
                                .execute(repetitions, count_successes)
                                .await
                                .map_err(|e| e.to_string())
                        };

                        if let Ok(history) = result {
                            if let Some(sender) = report_sender {
                                if sender.send(history).await.is_err() {
                                    eprintln!("Error sending execution history from non-blocking subflow.");
                                }
                            }
                        } else {
                            eprintln!("Non-blocking subflow execution failed.");
                        }
                    });
                }
            }
        });

        Self { sender }
    }

    pub async fn execute_subflow(
        &self,
        subflow: Arc<Mutex<Workflow>>,
        repetitions: Option<u32>,
        count_successes: bool,
        blocking: bool,
        report_sender: Option<mpsc::Sender<Vec<ExecutionHistory>>>,
    ) -> Option<Result<Vec<ExecutionHistory>, String>> {
        if blocking {
            let (responder, receiver) = oneshot::channel();
            let _ = self
                .sender
                .send(SubflowRequest {
                    subflow,
                    repetitions,
                    count_successes,
                    blocking,
                    responder: Some(responder),
                    report_sender,
                })
                .await;

            receiver.await.ok()
        } else {
            let _ = self
                .sender
                .send(SubflowRequest {
                    subflow,
                    repetitions,
                    count_successes,
                    blocking,
                    responder: None,
                    report_sender,
                })
                .await;

            None
        }
    }
}
