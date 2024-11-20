use crate::{
    adapters::{
        links::{
            conditions::{configure_new_condition, Condition, ConditionType},
            evaluations::{configure_new_evaluation, Evaluation, EvaluationType},
            fhe_gates::{configure_new_gate, FHEGate},
        },
        nodes::{
            agents::{self, Agent, LLMModel, Objective},
            connectors::{
                off_chain::{configure_new_offchain_connector, OffChainConnector},
                on_chain::{configure_new_onchain_connector, OnChainConnector},
            },
            listeners::{configure_new_listener, Listener, ListenerType},
        },
    },
    constants::NIBBLE_FACTORY_CONTRACT,
    encrypt::encrypt_with_public_key,
    ipfs::{IPFSClient, IPFSClientFactory, IPFSProvider},
    utils::{generate_unique_id, load_nibble_from_subgraph, load_workflow_from_subgraph},
    workflow::Workflow,
};
use ethers::{
    abi::{Abi, AbiDecode, Token, Tokenize},
    prelude::*,
    types::{Address, Eip1559TransactionRequest, NameOrAddress, U256},
};
use futures::stream::{self, StreamExt, TryStreamExt};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, error::Error, fs::File, io::Read, path::Path, sync::Arc, vec};

pub struct AdapterHandle<'a, T>
where
    T: Adaptable,
{
    pub nibble: &'a mut Nibble,
    pub adapter: T,
    pub adapter_type: Adapter,
}

pub trait Adaptable {
    fn name(&self) -> &str;
    fn id(&self) -> &Vec<u8>;
}

#[derive(Debug, Clone)]
pub enum Adapter {
    Condition,
    OffChainConnector,
    OnChainConnector,
    Listener,
    FHEGate,
    Agent,
    Evaluation,
}

impl ToString for Adapter {
    fn to_string(&self) -> String {
        match self {
            Adapter::Condition => "Condition".to_string(),
            Adapter::OffChainConnector => "OffChainConnector".to_string(),
            Adapter::OnChainConnector => "OnChainConnector".to_string(),
            Adapter::Listener => "Listener".to_string(),
            Adapter::FHEGate => "FHEGate".to_string(),
            Adapter::Agent => "Agent".to_string(),
            Adapter::Evaluation => "Evaluation".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContractInfo {
    pub name: String,
    pub address: Address,
}

#[derive(Debug, Clone)]
pub struct Nibble {
    pub agents: Vec<Agent>,
    pub conditions: Vec<Condition>,
    pub listeners: Vec<Listener>,
    pub fhe_gates: Vec<FHEGate>,
    pub evaluations: Vec<Evaluation>,
    pub onchain_connectors: Vec<OnChainConnector>,
    pub offchain_connectors: Vec<OffChainConnector>,
    pub saved_agents: Vec<Agent>,
    pub saved_conditions: Vec<Condition>,
    pub saved_listeners: Vec<Listener>,
    pub saved_fhe_gates: Vec<FHEGate>,
    pub saved_evaluations: Vec<Evaluation>,
    pub saved_onchain_connectors: Vec<OnChainConnector>,
    pub saved_offchain_connectors: Vec<OffChainConnector>,
    pub contracts: Vec<ContractInfo>,
    pub owner_wallet: LocalWallet,
    pub id: Option<Vec<u8>>,
    pub count: U256,
    pub provider: Provider<Http>,
    pub chain: Chain,
    pub ipfs_client: Arc<dyn IPFSClient + Send + Sync>,
    pub graph_api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModifyAdapters {
    pub conditions: Vec<ContractCondition>,
    pub listeners: Vec<ContractListener>,
    pub connectors: Vec<ContractConnector>,
    pub agents: Vec<ContractAgent>,
    pub evaluations: Vec<ContractEvaluation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemoveAdapters {
    pub conditions: Vec<Vec<u8>>,
    pub listeners: Vec<Vec<u8>>,
    pub connectors: Vec<Vec<u8>>,
    pub agents: Vec<Vec<u8>>,
    pub evaluations: Vec<Vec<u8>>,
}

impl Tokenize for ModifyAdapters {
    fn into_tokens(self) -> Vec<Token> {
        vec![
            Token::Array(
                self.conditions
                    .into_iter()
                    .map(|condition| {
                        Token::Tuple(vec![
                            Token::Bytes(condition.id),
                            Token::String(condition.metadata),
                            Token::Bool(condition.encrypted),
                        ])
                    })
                    .collect(),
            ),
            Token::Array(
                self.listeners
                    .into_iter()
                    .map(|listener| {
                        Token::Tuple(vec![
                            Token::Bytes(listener.id),
                            Token::String(listener.metadata),
                            Token::Bool(listener.encrypted),
                        ])
                    })
                    .collect(),
            ),
            Token::Array(
                self.connectors
                    .into_iter()
                    .map(|connector| {
                        Token::Tuple(vec![
                            Token::Bytes(connector.id),
                            Token::String(connector.metadata),
                            Token::Bool(connector.encrypted),
                            Token::Bool(connector.onChain),
                        ])
                    })
                    .collect(),
            ),
            Token::Array(
                self.agents
                    .into_iter()
                    .map(|agent| {
                        Token::Tuple(vec![
                            Token::Bytes(agent.id),
                            Token::String(agent.metadata),
                            Token::Address(agent.wallet),
                            Token::Bool(agent.encrypted),
                            Token::Bool(agent.writer),
                        ])
                    })
                    .collect(),
            ),
            Token::Array(
                self.evaluations
                    .into_iter()
                    .map(|evaluation| {
                        Token::Tuple(vec![
                            Token::Bytes(evaluation.id),
                            Token::String(evaluation.metadata),
                            Token::Bool(evaluation.encrypted),
                        ])
                    })
                    .collect(),
            ),
        ]
    }
}

impl Tokenize for RemoveAdapters {
    fn into_tokens(self) -> Vec<Token> {
        vec![
            Token::Array(self.conditions.into_iter().map(Token::Bytes).collect()),
            Token::Array(self.listeners.into_iter().map(Token::Bytes).collect()),
            Token::Array(self.connectors.into_iter().map(Token::Bytes).collect()),
            Token::Array(self.agents.into_iter().map(Token::Bytes).collect()),
            Token::Array(self.evaluations.into_iter().map(Token::Bytes).collect()),
        ]
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ContractCondition {
    pub id: Vec<u8>,
    pub metadata: String,
    pub encrypted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContractEvaluation {
    pub id: Vec<u8>,
    pub metadata: String,
    pub encrypted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContractListener {
    pub id: Vec<u8>,
    pub metadata: String,
    pub encrypted: bool,
}

#[derive(Debug, Clone, Serialize)]
#[allow(non_snake_case)]
pub struct ContractConnector {
    pub id: Vec<u8>,
    pub metadata: String,
    pub encrypted: bool,
    pub onChain: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContractAgent {
    pub id: Vec<u8>,
    pub metadata: String,
    pub wallet: Address,
    pub encrypted: bool,
    pub writer: bool,
}

enum Connector<'a> {
    OnChain(&'a OnChainConnector),
    OffChain(&'a OffChainConnector),
}

impl Nibble {
    pub fn new(
        owner_private_key: &str,
        rpc_url: &str,
        ipfs_provider: IPFSProvider,
        ipfs_config: HashMap<String, String>,
        chain: Chain,
        graph_api_key: Option<String>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            agents: vec![],
            contracts: vec![],
            owner_wallet: owner_private_key.parse()?,
            id: None,
            count: U256::from(0),
            fhe_gates: vec![],
            evaluations: vec![],
            onchain_connectors: vec![],
            offchain_connectors: vec![],
            conditions: vec![],
            listeners: vec![],
            saved_fhe_gates: vec![],
            saved_evaluations: vec![],
            saved_onchain_connectors: vec![],
            saved_offchain_connectors: vec![],
            saved_conditions: vec![],
            saved_listeners: vec![],
            saved_agents: vec![],
            provider: Provider::<Http>::try_from(rpc_url)?,
            chain,
            graph_api_key,
            ipfs_client: IPFSClientFactory::create_client(ipfs_provider, ipfs_config)?,
        })
    }

    pub fn add_listener(
        &mut self,
        name: &str,
        event_name: &str,
        listener_type: ListenerType,
        encrypted: bool,
    ) -> Result<AdapterHandle<'_, Listener>, Box<dyn Error + Send + Sync>> {
        let listener = configure_new_listener(
            name,
            event_name,
            listener_type,
            encrypted,
            &self.owner_wallet.address(),
        )?;
        self.listeners.push(listener.clone());
        Ok(AdapterHandle {
            nibble: self,
            adapter: listener,
            adapter_type: Adapter::Listener,
        })
    }

    pub fn add_condition(
        &mut self,
        name: &str,
        condition_type: ConditionType,
        condition_fn: fn(Value) -> bool,
        expected_value: Option<Value>,
        encrypted: bool,
    ) -> Result<AdapterHandle<'_, Condition>, Box<dyn Error + Send + Sync>> {
        let condition: Condition = configure_new_condition(
            name,
            condition_type,
            condition_fn,
            expected_value,
            encrypted,
            &self.owner_wallet.address(),
        )?;
        self.conditions.push(condition.clone());
        Ok(AdapterHandle {
            nibble: self,
            adapter: condition,
            adapter_type: Adapter::Condition,
        })
    }

    pub fn add_fhe_gate(
        &mut self,
        name: &str,
        key: &str,
        encrypted: bool,
    ) -> Result<AdapterHandle<'_, FHEGate>, Box<dyn Error + Send + Sync>> {
        let fhe_gate: FHEGate =
            configure_new_gate(name, key, encrypted, &self.owner_wallet.address())?;
        self.fhe_gates.push(fhe_gate.clone());
        Ok(AdapterHandle {
            nibble: self,
            adapter: fhe_gate,
            adapter_type: Adapter::FHEGate,
        })
    }

    pub fn add_evaluation(
        &mut self,
        name: &str,
        evaluation_type: EvaluationType,
        encrypted: bool,
    ) -> Result<AdapterHandle<'_, Evaluation>, Box<dyn Error + Send + Sync>> {
        let evaluation = configure_new_evaluation(
            name,
            evaluation_type,
            encrypted,
            &self.owner_wallet.address(),
        )?;

        self.evaluations.push(evaluation.clone());
        Ok(AdapterHandle {
            nibble: self,
            adapter: evaluation,
            adapter_type: Adapter::Evaluation,
        })
    }

    pub fn add_onchain_connector(
        &mut self,
        name: &str,
        address: Address,
        encrypted: bool,
    ) -> Result<AdapterHandle<'_, OnChainConnector>, Box<dyn Error + Send + Sync>> {
        let on_chain = configure_new_onchain_connector(
            name,
            address,
            encrypted,
            &self.owner_wallet.address(),
        )?;
        self.onchain_connectors.push(on_chain.clone());
        Ok(AdapterHandle {
            nibble: self,
            adapter_type: Adapter::OnChainConnector,
            adapter: on_chain,
        })
    }

    pub fn add_offchain_connector(
        &mut self,
        name: &str,
        api_url: &str,
        encrypted: bool,
        http_method: Method,
        headers: Option<HashMap<String, String>>,
        execution_fn: Option<
            Box<dyn Fn(Value) -> Result<Value, Box<dyn Error + Send + Sync>> + Send + Sync>,
        >,
    ) -> Result<AdapterHandle<'_, OffChainConnector>, Box<dyn Error + Send + Sync>> {
        let off_chain = configure_new_offchain_connector(
            name,
            api_url,
            encrypted,
            http_method,
            headers,
            execution_fn.map(|f| Arc::from(f)),
            &self.owner_wallet.address(),
        )?;

        self.offchain_connectors.push(off_chain.clone());
        Ok(AdapterHandle {
            nibble: self,
            adapter: off_chain,
            adapter_type: Adapter::OffChainConnector,
        })
    }

    pub fn add_agent(
        &mut self,
        name: &str,
        role: &str,
        personality: &str,
        system: &str,
        write_role: bool,
        admin_role: bool,
        model: LLMModel,
        encrypted: bool,
        agent_wallet: Option<&H160>,
        lens_account: Option<&str>,
        farcaster_account: Option<&str>,
        objectives: Vec<Objective>,
    ) -> Result<AdapterHandle<'_, Agent>, Box<dyn Error + Send + Sync>> {
        let agent = agents::configure_new_agent(
            name,
            role,
            personality,
            system,
            write_role,
            admin_role,
            encrypted,
            model,
            &self.owner_wallet.address(),
            agent_wallet,
            farcaster_account,
            lens_account,
            objectives,
        )?;

        self.agents.push(agent.clone());
        Ok(AdapterHandle {
            nibble: self,
            adapter: agent,
            adapter_type: Adapter::Agent,
        })
    }

    pub async fn create_nibble(&mut self) -> Result<Nibble, Box<dyn Error + Send + Sync>> {
        let client = SignerMiddleware::new(
            self.provider.clone(),
            self.owner_wallet.clone().with_chain_id(self.chain),
        );
        let client = Arc::new(client);

        let mut abi_file = File::open(Path::new("./abis/NibbleFactory.json"))?;
        let mut abi_content = String::new();
        abi_file.read_to_string(&mut abi_content)?;
        let abi = serde_json::from_str::<Abi>(&abi_content)?;

        let contract_instance = Contract::new(
            NIBBLE_FACTORY_CONTRACT.parse::<Address>().unwrap(),
            abi,
            client.clone(),
        );

        let method =
            contract_instance.method::<_, ([Address; 9], Vec<u8>, U256)>("deployFromFactory", {});

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
                        to: Some(NameOrAddress::Address(
                            NIBBLE_FACTORY_CONTRACT.parse::<Address>().unwrap(),
                        )),
                        gas: Some(gas_limit),
                        value: tx_request.value,
                        data: tx_request.data.clone(),
                        max_priority_fee_per_gas: Some(max_priority_fee),
                        max_fee_per_gas: Some(gas_price + max_priority_fee),
                        chain_id: Some(Chain::PolygonAmoy.into()),
                        ..Default::default()
                    };

                    let pending_tx = match cliente.send_transaction(req, None).await {
                        Ok(tx) => tx,
                        Err(e) => {
                            eprintln!("Error sending the transaction: {:?}", e);
                            return Err(e.into());
                        }
                    };

                    let receipt = match pending_tx.await {
                        Ok(Some(receipt)) => receipt,
                        Ok(None) => {
                            return Err("Transaction not recieved".into());
                        }
                        Err(e) => {
                            eprintln!("Error with the transaction: {:?}", e);
                            return Err(e.into());
                        }
                    };

                    if let Some(log) = receipt.logs.get(0) {
                        let log_data_bytes = log.data.0.clone();
                        let return_values: ([Address; 9], Vec<u8>, U256) =
                            <([Address; 9], Vec<u8>, U256)>::decode(&log_data_bytes)?;

                        self.contracts = vec![
                            ContractInfo {
                                name: "NibbleStorage".to_string(),
                                address: return_values.0[0],
                            },
                            ContractInfo {
                                name: "NibbleListeners".to_string(),
                                address: return_values.0[1],
                            },
                            ContractInfo {
                                name: "NibbleConditions".to_string(),
                                address: return_values.0[2],
                            },
                            ContractInfo {
                                name: "NibbleEvaluations".to_string(),
                                address: return_values.0[3],
                            },
                            ContractInfo {
                                name: "NibbleAgents".to_string(),
                                address: return_values.0[4],
                            },
                            ContractInfo {
                                name: "NibbleConnectors".to_string(),
                                address: return_values.0[5],
                            },
                            ContractInfo {
                                name: "NibbleFHEGates".to_string(),
                                address: return_values.0[6],
                            },
                            ContractInfo {
                                name: "NibbleAccessControl".to_string(),
                                address: return_values.0[7],
                            },
                            ContractInfo {
                                name: "NibbleWorkflows".to_string(),
                                address: return_values.0[8],
                            },
                        ];
                        self.id = Some(return_values.1);
                        self.count = return_values.2;

                        Ok(Nibble {
                            agents: self.agents.clone(),
                            conditions: self.conditions.clone(),
                            listeners: self.listeners.clone(),
                            fhe_gates: self.fhe_gates.clone(),
                            evaluations: self.evaluations.clone(),
                            onchain_connectors: self.onchain_connectors.clone(),
                            offchain_connectors: self.offchain_connectors.clone(),
                            contracts: self.contracts.clone(),
                            owner_wallet: self.owner_wallet.clone(),
                            id: self.id.clone(),
                            count: self.count.clone(),
                            provider: self.provider.clone(),
                            chain: self.chain.clone(),
                            saved_fhe_gates: vec![],
                            saved_evaluations: vec![],
                            saved_onchain_connectors: vec![],
                            saved_offchain_connectors: vec![],
                            saved_conditions: vec![],
                            saved_listeners: vec![],
                            saved_agents: vec![],
                            ipfs_client: self.ipfs_client.clone(),
                            graph_api_key: self.graph_api_key.clone(),
                        })
                    } else {
                        Err("No transaction logs received.".into())
                    }
                } else {
                    Err("EIP-1559 reference invalid.".into())
                }
            }
            Err(e) => {
                eprintln!(
                    "Error while preparing the method of deployFromFactory: {}",
                    e
                );
                Err(e.into())
            }
        }
    }

    pub async fn load_nibble(
        &mut self,
        id: Vec<u8>,
    ) -> Result<Nibble, Box<dyn Error + Send + Sync>> {
        let response = load_nibble_from_subgraph(
            id,
            self.graph_api_key.clone(),
            self.owner_wallet.clone(),
            self.provider.clone(),
        )
        .await?;
        self.contracts = response.contracts;
        self.saved_conditions = response.conditions;
        self.saved_listeners = response.listeners;
        self.saved_offchain_connectors = response.offchain_connectors;
        self.saved_onchain_connectors = response.onchain_connectors;
        self.saved_evaluations = response.evaluations;
        self.saved_agents = response.agents;
        self.saved_fhe_gates = response.fhe_gates;
        self.count = response.count;

        Ok(Nibble {
            fhe_gates: vec![],
            evaluations: vec![],
            onchain_connectors: vec![],
            offchain_connectors: vec![],
            conditions: vec![],
            listeners: vec![],
            agents: vec![],
            saved_agents: self.agents.clone(),
            saved_conditions: self.conditions.clone(),
            saved_listeners: self.listeners.clone(),
            saved_fhe_gates: self.fhe_gates.clone(),
            saved_evaluations: self.evaluations.clone(),
            saved_onchain_connectors: self.onchain_connectors.clone(),
            saved_offchain_connectors: self.offchain_connectors.clone(),
            contracts: self.contracts.clone(),
            owner_wallet: self.owner_wallet.clone(),
            id: self.id.clone(),
            count: self.count.clone(),
            provider: self.provider.clone(),
            chain: self.chain.clone(),
            ipfs_client: self.ipfs_client.clone(),
            graph_api_key: self.graph_api_key.clone(),
        })
    }

    pub async fn remove_adapters(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if self.contracts.len() < 1 {
            return Err("No contracts found. Load or create a Nibble.".into());
        }

        let client = SignerMiddleware::new(
            self.provider.clone(),
            self.owner_wallet.clone().with_chain_id(self.chain),
        );
        let client = Arc::new(client);

        let storage_contract_address = self
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

        let remove_adapters = self.build_remove_adapters()?;
        let method = contract_instance.method::<_, H256>("removeAdaptersBatch", remove_adapters);

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

        self.conditions.clear();
        self.listeners.clear();
        self.fhe_gates.clear();
        self.evaluations.clear();
        self.onchain_connectors.clear();
        self.offchain_connectors.clear();
        self.agents.clear();

        let response = load_nibble_from_subgraph(
            self.id.as_ref().unwrap().clone(),
            self.graph_api_key.clone(),
            self.owner_wallet.clone(),
            self.provider.clone(),
        )
        .await?;
        self.contracts = response.contracts;
        self.saved_conditions = response.conditions;
        self.saved_listeners = response.listeners;
        self.saved_offchain_connectors = response.offchain_connectors;
        self.saved_onchain_connectors = response.onchain_connectors;
        self.saved_evaluations = response.evaluations;
        self.saved_agents = response.agents;
        self.saved_fhe_gates = response.fhe_gates;
        self.count = response.count;

        Ok(())
    }

    pub async fn persist_adapters(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if self.contracts.len() < 1 {
            return Err("No contracts found. Load or create a Nibble.".into());
        }

        let client = SignerMiddleware::new(
            self.provider.clone(),
            self.owner_wallet.clone().with_chain_id(self.chain),
        );
        let client = Arc::new(client);

        let storage_contract_address = self
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

        let modify_adapters = self
            .build_modify_adapters(self.ipfs_client.as_ref())
            .await?;

        let method =
            contract_instance.method::<_, H256>("addOrModifyAdaptersBatch", modify_adapters);

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

        self.conditions.clear();
        self.listeners.clear();
        self.fhe_gates.clear();
        self.evaluations.clear();
        self.onchain_connectors.clear();
        self.offchain_connectors.clear();
        self.agents.clear();

        let response = load_nibble_from_subgraph(
            self.id.as_ref().unwrap().clone(),
            self.graph_api_key.clone(),
            self.owner_wallet.clone(),
            self.provider.clone(),
        )
        .await
        .map_err(|e| Box::<dyn Error + Send + Sync>::from(e))?;
        self.contracts = response.contracts;
        self.saved_conditions = response.conditions;
        self.saved_listeners = response.listeners;
        self.saved_offchain_connectors = response.offchain_connectors;
        self.saved_onchain_connectors = response.onchain_connectors;
        self.saved_evaluations = response.evaluations;
        self.saved_agents = response.agents;
        self.saved_fhe_gates = response.fhe_gates;
        self.count = response.count;

        Ok(())
    }

    pub fn create_workflow(&self, name: &str, encrypted: bool) -> Workflow {
        Workflow {
            id: generate_unique_id(&self.owner_wallet.address()),
            name: name.to_string(),
            nodes: HashMap::new(),
            links: HashMap::new(),
            nibble_context: Arc::new(self.clone()),
            encrypted,
            execution_history: Vec::new(),
        }
    }

    pub async fn load_workflow(
        &self,
        id: Vec<u8>,
    ) -> Result<Workflow, Box<dyn Error + Send + Sync>> {
        if self.contracts.len() < 1 {
            return Err("No contracts found. Load or create a Nibble firsty.".into());
        }

        let workflow = load_workflow_from_subgraph(
            id,
            self.id.as_ref().unwrap().clone(),
            self.graph_api_key.clone(),
        )
        .await?;

        Ok(Workflow {
            id: workflow.id,
            name: workflow.name,
            nodes: workflow.nodes,
            links: workflow.links,
            nibble_context: Arc::new(self.clone()),
            encrypted: workflow.encrypted,
            execution_history: workflow.execution_history
        })
    }

    fn build_remove_adapters(&self) -> Result<RemoveAdapters, Box<dyn Error + Send + Sync>> {
        Ok(RemoveAdapters {
            conditions: self
                .conditions
                .iter()
                .map(|condition| condition.id.clone())
                .collect(),
            listeners: self
                .listeners
                .iter()
                .map(|listener| listener.id.clone())
                .collect(),
            connectors: self
                .onchain_connectors
                .iter()
                .map(|c| Connector::OnChain(c))
                .chain(
                    self.offchain_connectors
                        .iter()
                        .map(|c| Connector::OffChain(c)),
                )
                .map(|connector| {
                    let id = match connector {
                        Connector::OnChain(on_chain) => &on_chain.id,
                        Connector::OffChain(off_chain) => &off_chain.id,
                    };
                    id.clone()
                })
                .collect(),
            agents: self.agents.iter().map(|agent| agent.id.clone()).collect(),
            evaluations: self
                .evaluations
                .iter()
                .map(|evaluation| evaluation.id.clone())
                .collect(),
        })
    }

    async fn build_modify_adapters(
        &self,
        ipfs_client: &dyn IPFSClient,
    ) -> Result<ModifyAdapters, Box<dyn Error + Send + Sync>> {
        Ok(ModifyAdapters {
            conditions: stream::iter(&self.conditions)
                .then(|condition| async {
                    let mut metadata = serde_json::to_vec(&condition.to_json())?;

                    if condition.encrypted {
                        metadata = encrypt_with_public_key(metadata, self.owner_wallet.clone())?;
                    }
                    let ipfs_hash = ipfs_client.upload(metadata).await?;
                    Ok::<ContractCondition, Box<dyn Error + Send + Sync>>(ContractCondition {
                        id: condition.id().to_vec(),
                        metadata: ipfs_hash,
                        encrypted: condition.encrypted,
                    })
                })
                .try_collect::<Vec<_>>()
                .await?,
            listeners: stream::iter(&self.listeners)
                .then(|listener| async {
                    let mut metadata = serde_json::to_vec(&listener.to_json())?;

                    if listener.encrypted {
                        metadata = encrypt_with_public_key(metadata, self.owner_wallet.clone())?;
                    }

                    let ipfs_hash = ipfs_client.upload(metadata).await?;
                    Ok::<ContractListener, Box<dyn Error + Send + Sync>>(ContractListener {
                        id: listener.id().to_vec(),
                        metadata: ipfs_hash,
                        encrypted: listener.encrypted,
                    })
                })
                .try_collect::<Vec<_>>()
                .await?,
            connectors: stream::iter(
                self.onchain_connectors
                    .iter()
                    .map(|c| Connector::OnChain(c))
                    .chain(
                        self.offchain_connectors
                            .iter()
                            .map(|c| Connector::OffChain(c)),
                    ),
            )
            .then(|connector| async move {
                let (mut metadata, is_onchain) = match connector {
                    Connector::OnChain(on_chain) => (
                        serde_json::to_vec(&on_chain.to_json())
                            .map_err(|e| format!("Failed to serialize OnChainConnector: {}", e))?,
                        true,
                    ),
                    Connector::OffChain(off_chain) => (
                        serde_json::to_vec(&off_chain.to_json())
                            .map_err(|e| format!("Failed to serialize OffChainConnector: {}", e))?,
                        false,
                    ),
                };
                let encrypted = match connector {
                    Connector::OnChain(on_chain) => &on_chain.encrypted,
                    Connector::OffChain(off_chain) => &off_chain.encrypted,
                };

                if encrypted.clone() {
                    metadata = encrypt_with_public_key(metadata, self.owner_wallet.clone())?;
                }

                let ipfs_hash = ipfs_client.upload(metadata).await?;

                let id = match connector {
                    Connector::OnChain(on_chain) => &on_chain.id,
                    Connector::OffChain(off_chain) => &off_chain.id,
                };

                Ok::<ContractConnector, Box<dyn Error + Send + Sync>>(ContractConnector {
                    id: id.clone(),
                    metadata: ipfs_hash,
                    encrypted: encrypted.clone(),
                    onChain: is_onchain,
                })
            })
            .try_collect::<Vec<_>>()
            .await?,
            agents: stream::iter(&self.agents)
                .then(|agent| async {
                    let mut metadata = serde_json::to_vec(&agent.to_json())?;

                    if agent.encrypted {
                        metadata = encrypt_with_public_key(metadata, self.owner_wallet.clone())?;
                    }

                    let ipfs_hash = ipfs_client.upload(metadata).await?;
                    Ok::<ContractAgent, Box<dyn Error + Send + Sync>>(ContractAgent {
                        id: agent.id().to_vec(),
                        metadata: ipfs_hash,
                        encrypted: agent.encrypted,
                        wallet: agent.wallet.address(),
                        writer: agent.write_role || agent.admin_role,
                    })
                })
                .try_collect::<Vec<_>>()
                .await?,
            evaluations: stream::iter(&self.evaluations)
                .then(|evaluation| async {
                    let mut metadata = serde_json::to_vec(&evaluation.to_json())?;
                    if evaluation.encrypted {
                        metadata = encrypt_with_public_key(metadata, self.owner_wallet.clone())?;
                    }

                    let ipfs_hash = ipfs_client.upload(metadata).await?;
                    Ok::<ContractEvaluation, Box<dyn Error + Send + Sync>>(ContractEvaluation {
                        id: evaluation.id().to_vec(),
                        metadata: ipfs_hash,
                        encrypted: evaluation.encrypted,
                    })
                })
                .try_collect::<Vec<_>>()
                .await?,
        })
    }
}

impl<'a, T> AdapterHandle<'a, T>
where
    T: Adaptable + Serialize + std::fmt::Debug,
{
    pub async fn persist_adapter(self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let client = SignerMiddleware::new(
            self.nibble.provider.clone(),
            self.nibble
                .owner_wallet
                .clone()
                .with_chain_id(self.nibble.chain),
        );
        let client = Arc::new(client);

        let contract_address = match self.adapter_type {
            Adapter::Condition => {
                self.nibble
                    .contracts
                    .iter()
                    .find(|c| c.name == "NibbleConditions")
                    .ok_or("Condition contract not found")?
                    .address
            }
            Adapter::Listener => {
                self.nibble
                    .contracts
                    .iter()
                    .find(|c| c.name == "NibbleListeners")
                    .ok_or("Listener contract not found")?
                    .address
            }
            Adapter::FHEGate => {
                self.nibble
                    .contracts
                    .iter()
                    .find(|c| c.name == "NibbleFHEGates")
                    .ok_or("FHEGate contract not found")?
                    .address
            }
            Adapter::Evaluation => {
                self.nibble
                    .contracts
                    .iter()
                    .find(|c| c.name == "NibbleEvaluations")
                    .ok_or("Evaluation contract not found")?
                    .address
            }
            Adapter::OnChainConnector => {
                self.nibble
                    .contracts
                    .iter()
                    .find(|c| c.name == "NibbleConnectors")
                    .ok_or("OnChainConnector contract not found")?
                    .address
            }
            Adapter::OffChainConnector => {
                self.nibble
                    .contracts
                    .iter()
                    .find(|c| c.name == "NibbleConnectors")
                    .ok_or("OffChainConnector contract not found")?
                    .address
            }
            Adapter::Agent => {
                self.nibble
                    .contracts
                    .iter()
                    .find(|c| c.name == "NibbleAgents")
                    .ok_or("Agent contract not found")?
                    .address
            }
        };

        let serialized_adapter = serde_json::to_vec(&self.adapter)?;

        let contract_instance = Contract::new(contract_address, Abi::default(), client.clone());

        let method_name = match self.adapter_type {
            Adapter::Condition => "addOrModifyConditionsBatch",
            Adapter::Listener => "addOrModifyListenersBatch",
            Adapter::FHEGate => "addOrModifyFHEGatesBatch",
            Adapter::Evaluation => "addOrModifyEvaluationsBatch",
            Adapter::OnChainConnector => "addOrModifyConnectorsBatch",
            Adapter::OffChainConnector => "addOrModifyConnectorsBatch",
            Adapter::Agent => "addOrModifyAgentsBatch",
        };

        let method = contract_instance.method::<_, H256>(&method_name, vec![serialized_adapter]);

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
                        to: Some(NameOrAddress::Address(contract_address)),
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
                eprintln!("Error while preparing the method of {}: {}", method_name, e);
                return Err(e.into());
            }
        }

        match self.adapter_type {
            Adapter::Condition => {
                if let Some(index) = self
                    .nibble
                    .conditions
                    .iter()
                    .position(|c| c.name() == self.adapter.name())
                {
                    self.nibble.conditions.remove(index);
                }
            }
            Adapter::Listener => {
                if let Some(index) = self
                    .nibble
                    .listeners
                    .iter()
                    .position(|l| l.name() == self.adapter.name())
                {
                    self.nibble.listeners.remove(index);
                }
            }
            Adapter::FHEGate => {
                if let Some(index) = self
                    .nibble
                    .fhe_gates
                    .iter()
                    .position(|g| g.name() == self.adapter.name())
                {
                    self.nibble.fhe_gates.remove(index);
                }
            }
            Adapter::Evaluation => {
                if let Some(index) = self
                    .nibble
                    .evaluations
                    .iter()
                    .position(|e| e.name() == self.adapter.name())
                {
                    self.nibble.evaluations.remove(index);
                }
            }
            Adapter::OnChainConnector => {
                if let Some(index) = self
                    .nibble
                    .onchain_connectors
                    .iter()
                    .position(|c| c.name() == self.adapter.name())
                {
                    self.nibble.onchain_connectors.remove(index);
                }
            }
            Adapter::OffChainConnector => {
                if let Some(index) = self
                    .nibble
                    .offchain_connectors
                    .iter()
                    .position(|c| c.name() == self.adapter.name())
                {
                    self.nibble.offchain_connectors.remove(index);
                }
            }
            Adapter::Agent => {
                if let Some(index) = self
                    .nibble
                    .agents
                    .iter()
                    .position(|a| a.name() == self.adapter.name())
                {
                    self.nibble.agents.remove(index);
                }
            }
        };

        let response = load_nibble_from_subgraph(
            self.nibble.id.as_ref().unwrap().clone(),
            self.nibble.graph_api_key.clone(),
            self.nibble.owner_wallet.clone(),
            self.nibble.provider.clone(),
        )
        .await?;
        self.nibble.contracts = response.contracts;
        self.nibble.saved_conditions = response.conditions;
        self.nibble.saved_listeners = response.listeners;
        self.nibble.saved_offchain_connectors = response.offchain_connectors;
        self.nibble.saved_onchain_connectors = response.onchain_connectors;
        self.nibble.saved_evaluations = response.evaluations;
        self.nibble.saved_agents = response.agents;
        self.nibble.saved_fhe_gates = response.fhe_gates;
        self.nibble.count = response.count;

        Ok(())
    }

    pub async fn remove_adapter(self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let client = SignerMiddleware::new(
            self.nibble.provider.clone(),
            self.nibble
                .owner_wallet
                .clone()
                .with_chain_id(self.nibble.chain),
        );
        let client = Arc::new(client);

        let contract_address = match self.adapter_type {
            Adapter::Condition => {
                self.nibble
                    .contracts
                    .iter()
                    .find(|c| c.name == "NibbleConditions")
                    .ok_or("Condition contract not found")?
                    .address
            }
            Adapter::Listener => {
                self.nibble
                    .contracts
                    .iter()
                    .find(|c| c.name == "NibbleListeners")
                    .ok_or("Listener contract not found")?
                    .address
            }
            Adapter::FHEGate => {
                self.nibble
                    .contracts
                    .iter()
                    .find(|c| c.name == "NibbleFHEGates")
                    .ok_or("FHEGate contract not found")?
                    .address
            }
            Adapter::Evaluation => {
                self.nibble
                    .contracts
                    .iter()
                    .find(|c| c.name == "NibbleEvaluations")
                    .ok_or("Evaluation contract not found")?
                    .address
            }
            Adapter::OnChainConnector => {
                self.nibble
                    .contracts
                    .iter()
                    .find(|c| c.name == "NibbleConnectors")
                    .ok_or("OnChainConnector contract not found")?
                    .address
            }
            Adapter::OffChainConnector => {
                self.nibble
                    .contracts
                    .iter()
                    .find(|c| c.name == "NibbleConnectors")
                    .ok_or("OffChainConnector contract not found")?
                    .address
            }
            Adapter::Agent => {
                self.nibble
                    .contracts
                    .iter()
                    .find(|c| c.name == "NibbleAgents")
                    .ok_or("Agent contract not found")?
                    .address
            }
        };

        let contract_instance = Contract::new(contract_address, Abi::default(), client.clone());

        let method_name = match self.adapter_type {
            Adapter::Condition => "removeListenersBatch",
            Adapter::Listener => "removeListenersBatch",
            Adapter::FHEGate => "removeFHEGatesBatch",
            Adapter::Evaluation => "removeEvaluationsBatch",
            Adapter::OnChainConnector => "removeConnectorsBatch",
            Adapter::OffChainConnector => "removeConnectorsBatch",
            Adapter::Agent => "removeAgentsBatch",
        };

        let method =
            contract_instance.method::<_, H256>(&method_name, vec![self.adapter.id().clone()]);

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
                        to: Some(NameOrAddress::Address(contract_address)),
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
                eprintln!("Error while preparing the method of {}: {}", method_name, e);
                return Err(e.into());
            }
        }

        match self.adapter_type {
            Adapter::Condition => {
                if let Some(index) = self
                    .nibble
                    .conditions
                    .iter()
                    .position(|c| c.name() == self.adapter.name())
                {
                    self.nibble.conditions.remove(index);
                }
            }
            Adapter::Listener => {
                if let Some(index) = self
                    .nibble
                    .listeners
                    .iter()
                    .position(|l| l.name() == self.adapter.name())
                {
                    self.nibble.listeners.remove(index);
                }
            }
            Adapter::FHEGate => {
                if let Some(index) = self
                    .nibble
                    .fhe_gates
                    .iter()
                    .position(|g| g.name() == self.adapter.name())
                {
                    self.nibble.fhe_gates.remove(index);
                }
            }
            Adapter::Evaluation => {
                if let Some(index) = self
                    .nibble
                    .evaluations
                    .iter()
                    .position(|e| e.name() == self.adapter.name())
                {
                    self.nibble.evaluations.remove(index);
                }
            }
            Adapter::OnChainConnector => {
                if let Some(index) = self
                    .nibble
                    .onchain_connectors
                    .iter()
                    .position(|c| c.name() == self.adapter.name())
                {
                    self.nibble.onchain_connectors.remove(index);
                }
            }
            Adapter::OffChainConnector => {
                if let Some(index) = self
                    .nibble
                    .offchain_connectors
                    .iter()
                    .position(|c| c.name() == self.adapter.name())
                {
                    self.nibble.offchain_connectors.remove(index);
                }
            }
            Adapter::Agent => {
                if let Some(index) = self
                    .nibble
                    .agents
                    .iter()
                    .position(|a| a.name() == self.adapter.name())
                {
                    self.nibble.agents.remove(index);
                }
            }
        };

        let response = load_nibble_from_subgraph(
            self.nibble.id.as_ref().unwrap().clone(),
            self.nibble.graph_api_key.clone(),
            self.nibble.owner_wallet.clone(),
            self.nibble.provider.clone(),
        )
        .await?;
        self.nibble.contracts = response.contracts;
        self.nibble.saved_conditions = response.conditions;
        self.nibble.saved_listeners = response.listeners;
        self.nibble.saved_offchain_connectors = response.offchain_connectors;
        self.nibble.saved_onchain_connectors = response.onchain_connectors;
        self.nibble.saved_evaluations = response.evaluations;
        self.nibble.saved_agents = response.agents;
        self.nibble.saved_fhe_gates = response.fhe_gates;
        self.nibble.count = response.count;

        Ok(())
    }
}
