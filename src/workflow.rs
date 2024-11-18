use crate::{
    ipfs::IPFSClient, encrypt::encrypt_with_public_key, nibble::Nibble, utils::generate_unique_id,
};
use ethers::{
    abi::{Abi, Token, Tokenize},
    middleware::SignerMiddleware,
    prelude::*,
    utils::hex,
};
use serde::Serialize;
use serde_json::{Map, Value};

use std::{collections::HashMap, error::Error, fs::File, io::Read, path::Path, sync::Arc};

#[derive(Debug, Clone)]
pub enum NodeAdapter {
    OffChainConnector,
    OnChainConnector,
    Agent,
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
    pub nodes: Vec<WorkflowNode>,
    pub links: Vec<WorkflowLink>,
    pub nibble_context: Arc<Nibble>,
    pub dependent_workflows: Vec<Vec<u8>>,
    pub encrypted: bool,
}

#[derive(Debug, Clone)]
pub struct WorkflowNode {
    pub id: Vec<u8>,
    pub adapter_type: NodeAdapter,
    pub adapter_id: Vec<u8>,
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
pub struct WorkflowLink {
    pub id: Vec<u8>,
    pub from_node: Vec<u8>,
    pub to_node: Vec<u8>,
    pub adapter_id: Vec<u8>,
    pub adapter_type: LinkAdapter,
}

impl WorkflowLink {
    pub fn to_json(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("id".to_string(), Value::String(hex::encode(&self.id)));
        map.insert(
            "from_node".to_string(),
            Value::String(hex::encode(&self.from_node)),
        );
        map.insert(
            "to_node".to_string(),
            Value::String(hex::encode(&self.to_node)),
        );
        map.insert(
            "adapter_id".to_string(),
            Value::String(hex::encode(&self.to_node)),
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
    pub fn add_node(&mut self, adapter_id: Vec<u8>) -> Result<(), String> {
        let adapter_type = if self
            .nibble_context
            .agents
            .iter()
            .chain(self.nibble_context.saved_agents.iter())
            .find(|a| a.id == adapter_id)
            .is_some()
        {
            NodeAdapter::Agent
        } else if self
            .nibble_context
            .offchain_connectors
            .iter()
            .chain(self.nibble_context.saved_offchain_connectors.iter())
            .find(|c| c.id == adapter_id)
            .is_some()
        {
            NodeAdapter::OffChainConnector
        } else if self
            .nibble_context
            .onchain_connectors
            .iter()
            .chain(self.nibble_context.saved_onchain_connectors.iter())
            .find(|c| c.id == adapter_id)
            .is_some()
        {
            NodeAdapter::OnChainConnector
        } else {
            return Err("Adapter with the given ID not found".into());
        };

        self.nodes.push(WorkflowNode {
            id: generate_unique_id(&self.nibble_context.owner_wallet.address()),
            adapter_id,
            adapter_type,
        });
        Ok(())
    }

    pub fn add_link(
        &mut self,
        from_node: Vec<u8>,
        to_node: Vec<u8>,
        adapter_id: Vec<u8>,
    ) -> Result<(), String> {
        let adapter_type = if self
            .nibble_context
            .conditions
            .iter()
            .chain(self.nibble_context.saved_conditions.iter())
            .find(|l| l.id == adapter_id)
            .is_some()
        {
            LinkAdapter::Condition
        } else if self
            .nibble_context
            .listeners
            .iter()
            .chain(self.nibble_context.saved_listeners.iter())
            .find(|l| l.id == adapter_id)
            .is_some()
        {
            LinkAdapter::Listener
        } else if self
            .nibble_context
            .evaluations
            .iter()
            .chain(self.nibble_context.saved_evaluations.iter())
            .find(|a| a.id == adapter_id)
            .is_some()
        {
            LinkAdapter::Evaluation
        } else if self
            .nibble_context
            .fhe_gates
            .iter()
            .chain(self.nibble_context.saved_fhe_gates.iter())
            .find(|c| c.id == adapter_id)
            .is_some()
        {
            LinkAdapter::FHEGate
        } else {
            return Err("Adapter with the given ID not found".into());
        };

        self.links.push(WorkflowLink {
            id: generate_unique_id(&self.nibble_context.owner_wallet.address()),
            from_node,
            to_node,
            adapter_id,
            adapter_type,
        });
        Ok(())
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

    async fn execute(&self, context: &mut ExecutionContext) -> Result<(), Box<dyn Error + Send + Sync>> {
        let execution_order = self.topological_sort()?;

        for node_id in execution_order {
            let node = self
                .nodes
                .iter()
                .find(|n| n.id == node_id)
                .ok_or("Node not found in execution order")?;

            for link in self.links.iter().filter(|link| link.to_node == node_id) {
                match link.adapter_type {
                    LinkAdapter::Listener => {
                        let listener = self
                            .nibble_context
                            .listeners
                            .iter()
                            .find(|l| l.id == link.adapter_id)
                            .ok_or("Listener not found")?
                            .clone();

                        let workflow = Arc::new(self.clone());
                        let to_node = link.to_node.clone();

                        tokio::spawn(async move {
                            if let Err(e) = listener.listen_and_trigger(workflow, to_node).await {
                                eprintln!("Error in listener: {}", e);
                            }
                        });
                    }
                    LinkAdapter::Condition => {
                        let condition = self
                            .nibble_context
                            .conditions
                            .iter()
                            .chain(self.nibble_context.saved_conditions.iter())
                            .find(|c| c.id == link.adapter_id)
                            .ok_or("Condition not found")?;

                        let is_valid = condition.check_condition(&self.nibble_context).await?;

                        if !is_valid {
                            return Err(format!(
                            "Node {:?} execution failed due to failed condition on link from {:?} to {:?}",
                            node.adapter_id,
                            link.from_node,
                            link.to_node
                        )
                        .into());
                        }
                    }
                    LinkAdapter::Evaluation => {
                        let evaluation = self
                            .nibble_context
                            .evaluations
                            .iter()
                            .chain(self.nibble_context.saved_evaluations.iter())
                            .find(|c| c.id == link.adapter_id)
                            .ok_or("Evaluation not found")?;

                        let is_valid = evaluation.check_evaluation().await?;

                        if !is_valid {
                            return Err(format!(
                        "Node {:?} execution failed due to failed evaluation on link from {:?} to {:?}",
                        node.adapter_id,
                        link.from_node,
                        link.to_node
                    )
                    .into());
                        }
                    }
                    LinkAdapter::FHEGate => {
                        let fhe_gate = self
                            .nibble_context
                            .fhe_gates
                            .iter()
                            .chain(self.nibble_context.saved_fhe_gates.iter())
                            .find(|c| c.id == link.adapter_id)
                            .ok_or("FHE gate not found")?;

                        let is_valid = fhe_gate.check_fhe_gate().await?;

                        if !is_valid {
                            return Err(format!(
                    "Node {:?} FHE gate failed due to failed evaluation on link from {:?} to {:?}",
                    node.adapter_id,
                    link.from_node,
                    link.to_node
                )
                            .into());
                        }
                    }
                }
            }

            match node.adapter_type {
                NodeAdapter::Agent => {
                    let agent = self
                        .nibble_context
                        .agents
                        .iter()
                        .find(|a| a.id == node.adapter_id)
                        .ok_or("Agent not found")?;

                    let input_prompt = "Your input prompt for the agent";
                    let output = agent.execute_agent(input_prompt).await?;
                    println!("Agent execution result: {}", output);
                }
                NodeAdapter::OnChainConnector => {
                    let connector = self
                        .nibble_context
                        .onchain_connectors
                        .iter()
                        .find(|c| c.id == node.adapter_id)
                        .ok_or("OnChainConnector not found")?;

                    let client = Arc::new(SignerMiddleware::new(
                        self.nibble_context.provider.clone(),
                        self.nibble_context.owner_wallet.clone(),
                    ));

                    connector.execute_onchain_connector(client).await?;
                }
                NodeAdapter::OffChainConnector => {
                    let connector = self
                        .nibble_context
                        .offchain_connectors
                        .iter()
                        .find(|c| c.id == node.adapter_id)
                        .ok_or("OffChainConnector not found")?;

                    let response = connector.execute_offchain_connector(None).await?;
                    println!("OffChainConnector response: {:?}", response);
                }
            }
        }

        Ok(())
    }

    pub async fn execute_node(&self, node_id: Vec<u8>) -> Result<(), Box<dyn Error + Send + Sync>> {
        let node = self
            .nodes
            .iter()
            .find(|n| n.id == node_id)
            .ok_or("Node not found for execution")?;

        match node.adapter_type {
            NodeAdapter::Agent => {
                let agent = self
                    .nibble_context
                    .agents
                    .iter()
                    .find(|a| a.id == node.adapter_id)
                    .ok_or("Agent not found")?;
                let input_prompt = "Your input prompt for the agent";
                let output = agent.execute_agent(input_prompt).await?;
                println!("Agent execution result: {}", output);
            }
            NodeAdapter::OnChainConnector => {
                let connector = self
                    .nibble_context
                    .onchain_connectors
                    .iter()
                    .find(|c| c.id == node.adapter_id)
                    .ok_or("OnChainConnector not found")?;
                let client = Arc::new(SignerMiddleware::new(
                    self.nibble_context.provider.clone(),
                    self.nibble_context.owner_wallet.clone(),
                ));
                connector.execute_onchain_connector(client).await?;
            }
            NodeAdapter::OffChainConnector => {
                let connector = self
                    .nibble_context
                    .offchain_connectors
                    .iter()
                    .find(|c| c.id == node.adapter_id)
                    .ok_or("OffChainConnector not found")?;
                let response = connector.execute_offchain_connector(None).await?;
                println!("OffChainConnector response: {:?}", response);
            }
        }

        Ok(())
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
                    .map(|link| Value::Object(link.to_json()))
                    .collect(),
            ),
        );
        metadata_map.insert(
            "nodes".to_string(),
            Value::Array(
                self.nodes
                    .iter()
                    .map(|node| Value::Object(node.to_json()))
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

        for node in &self.nodes {
            in_degree.insert(node.id.clone(), 0);
            graph.insert(node.id.clone(), Vec::new());
        }
        for link in &self.links {
            graph
                .entry(link.from_node.clone())
                .or_default()
                .push(link.to_node.clone());
            *in_degree.entry(link.to_node.clone()).or_default() += 1;
        }

        let mut stack: Vec<Vec<u8>> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();
        let mut sorted = Vec::new();

        while let Some(node) = stack.pop() {
            sorted.push(node.clone());
            if let Some(neighbors) = graph.get(&node) {
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

        if sorted.len() != self.nodes.len() {
            return Err("Cyclic dependency detected in workflow".to_string());
        }

        Ok(sorted)
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub data_store: HashMap<String, Value>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            data_store: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: &str, value: Value) {
        self.data_store.insert(key.to_string(), value);
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data_store.get(key)
    }
}

pub struct WorkflowManager {
    pub workflows: HashMap<Vec<u8>, Workflow>,
}

impl WorkflowManager {
    pub async fn execute_workflow(
        &mut self,
        workflow_id: Vec<u8>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut context = ExecutionContext::new();

        if let Some(workflow) = self.workflows.get(&workflow_id) {
            workflow.execute(&mut context).await?;

            for dependent_id in &workflow.dependent_workflows {
                if let Some(dependent_workflow) = self.workflows.get(dependent_id) {
                    dependent_workflow.execute(&mut context).await?;
                }
            }
        } else {
            return Err(format!("Workflow with ID {:?} not found", workflow_id).into());
        }

        Ok(())
    }
}
