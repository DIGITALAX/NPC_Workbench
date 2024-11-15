use crate::{
    adapters::{
        agents::{self, Agent, LLMModel},
        conditions::{configure_new_condition, Condition, ConditionType},
        connectors::{
            off_chain::{configure_new_offchain_connector, OffChainConnector},
            on_chain::{configure_new_onchain_connector, OnChainConnector},
        },
        evaluations::{configure_new_evaluation, Evaluation, EvaluationType},
        fhe_gates::{configure_new_gate, FHEGate},
        listeners::{configure_new_listener, Listener, ListenerType},
    },
    constants::NIBBLE_FACTORY_CONTRACT,
    ipfs::{IPFSClient, IPFSClientFactory, IPFSProvider},
    utils::load_nibble_from_subgraph,
};
use ethers::{
    abi::{Abi, AbiDecode, Token, Tokenize},
    prelude::*,
    types::{Address, Eip1559TransactionRequest, NameOrAddress, U256},
};
use futures::stream::{self, StreamExt, TryStreamExt};
use reqwest::Method;
use serde::Serialize;
use serde_json::Value;
use std::{collections::HashMap, error::Error, fs::File, io::Read, path::Path, sync::Arc};
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

#[derive(Debug)]
pub enum Adapter {
    Condition,
    OffChainConnector,
    OnChainConnector,
    Listener,
    FHEGate,
    Agent,
    Evaluation,
}

#[derive(Debug, Clone)]
pub struct ContractInfo {
    pub name: String,
    pub address: Address,
}

#[derive(Debug)]
pub struct Nibble {
    pub agents: Vec<Agent>,
    pub conditions: Vec<Condition>,
    pub listeners: Vec<Listener>,
    pub fhe_gates: Vec<FHEGate>,
    pub evaluations: Vec<Evaluation>,
    pub onchain_connectors: Vec<OnChainConnector>,
    pub offchain_connectors: Vec<OffChainConnector>,
    pub contracts: Vec<ContractInfo>,
    pub owner_wallet: LocalWallet,
    pub id: Option<Vec<u8>>,
    pub count: U256,
    pub provider: Provider<Http>,
    pub chain: Chain,
    pub ipfs_client: Arc<dyn IPFSClient>,
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
    ) -> Result<Self, Box<dyn Error>> {
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
            provider: Provider::<Http>::try_from(rpc_url)?,
            chain,
            ipfs_client: IPFSClientFactory::create_client(ipfs_provider, ipfs_config)?,
        })
    }

    pub fn add_listener(
        &mut self,
        name: &str,
        event_name: &str,
        listener_type: ListenerType,
        condition_fn: fn(Value) -> bool,
        expected_value: Option<Value>,
        public: bool,
    ) -> Result<(), Box<dyn Error>> {
        configure_new_listener(
            self,
            name,
            event_name,
            listener_type,
            condition_fn,
            expected_value,
            public,
        )
    }

    pub fn add_condition(
        &mut self,
        name: &str,
        condition_type: ConditionType,
        condition_fn: fn(Value) -> bool,
        expected_value: Option<Value>,
        public: bool,
    ) -> Result<AdapterHandle<'_, Condition>, Box<dyn Error>> {
        let condition: Condition = configure_new_condition(
            self,
            name,
            condition_type,
            condition_fn,
            expected_value,
            public,
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
        public: bool,
    ) -> Result<AdapterHandle<'_, FHEGate>, Box<dyn Error>> {
        let fhe_gate: FHEGate = configure_new_gate(self, name, key, public)?;
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
        public: bool,
    ) -> Result<AdapterHandle<'_, Evaluation>, Box<dyn Error>> {
        let evaluation = configure_new_evaluation(self, name, evaluation_type, public)?;

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
        public: bool,
    ) -> Result<AdapterHandle<'_, OnChainConnector>, Box<dyn Error>> {
        let on_chain = configure_new_onchain_connector(self, name, address, public)?;
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
        public: bool,
        http_method: Method,
        headers: Option<HashMap<String, String>>,
        execution_fn: Option<Box<dyn Fn(Value) -> Result<Value, Box<dyn Error>> + Send + Sync>>,
    ) -> Result<AdapterHandle<'_, OffChainConnector>, Box<dyn Error>> {
        let off_chain = configure_new_offchain_connector(
            self,
            name,
            api_url,
            public,
            http_method,
            headers,
            execution_fn.map(|f| Arc::from(f)),
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
        token_role: bool,
        model: LLMModel,
    ) -> Result<AdapterHandle<'_, Agent>, Box<dyn Error>> {
        let agent = agents::configure_new_agent(
            self,
            name,
            role,
            personality,
            system,
            write_role,
            admin_role,
            token_role,
            model,
        )?;

        self.agents.push(agent.clone());
        Ok(AdapterHandle {
            nibble: self,
            adapter: agent,
            adapter_type: Adapter::Agent,
        })
    }

    pub async fn create_nibble(&mut self) -> Result<Nibble, Box<dyn Error>> {
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
            contract_instance.method::<_, ([Address; 7], Vec<u8>, U256)>("deployFromFactory", {});

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
                        let return_values: ([Address; 7], Vec<u8>, U256) =
                            <([Address; 7], Vec<u8>, U256)>::decode(&log_data_bytes)?;

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
                                name: "NibbleAccessControl".to_string(),
                                address: return_values.0[6],
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
                            ipfs_client: self.ipfs_client.clone(),
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

    pub async fn load_nibble(&mut self, id: Vec<u8>) -> Result<Nibble, Box<dyn Error>> {
        let response = load_nibble_from_subgraph(id).await?;
        self.contracts = response.contracts;
        self.conditions = response.conditions;
        self.listeners = response.listeners;
        self.offchain_connectors = response.offchain_connectors;
        self.onchain_connectors = response.onchain_connectors;
        self.evaluations = response.evaluations;
        self.agents = response.agents;
        self.fhe_gates = response.fhe_gates;
        self.count = response.count;

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
            ipfs_client: self.ipfs_client.clone(),
        })
    }

    pub async fn remove_adapters(&mut self) -> Result<(), Box<dyn Error>> {
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

                    let pending_tx = cliente.send_transaction(req, None).await.map_err(|e| {
                        eprintln!("Error sending the transaction: {:?}", e);
                        Box::<dyn std::error::Error>::from(format!(
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

        Ok(())
    }

    pub async fn persist_adapters(&mut self) -> Result<(), Box<dyn Error>> {
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

                    let pending_tx = cliente.send_transaction(req, None).await.map_err(|e| {
                        eprintln!("Error sending the transaction: {:?}", e);
                        Box::<dyn std::error::Error>::from(format!(
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

        Ok(())
    }

    fn build_remove_adapters(&self) -> Result<RemoveAdapters, Box<dyn Error>> {
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
    ) -> Result<ModifyAdapters, Box<dyn Error>> {
        Ok(ModifyAdapters {
            conditions: stream::iter(&self.conditions)
                .then(|condition| async {
                    let metadata = serde_json::to_vec(&condition.to_json())?;
                    let ipfs_hash = ipfs_client.upload(metadata).await?;
                    Ok::<ContractCondition, Box<dyn std::error::Error>>(ContractCondition {
                        id: condition.id().to_vec(),
                        metadata: ipfs_hash,
                        encrypted: false,
                    })
                })
                .try_collect::<Vec<_>>()
                .await?,
            listeners: stream::iter(&self.listeners)
                .then(|listener| async {
                    let metadata = serde_json::to_vec(&listener.to_json())?;
                    let ipfs_hash = ipfs_client.upload(metadata).await?;
                    Ok::<ContractListener, Box<dyn std::error::Error>>(ContractListener {
                        id: listener.id().to_vec(),
                        metadata: ipfs_hash,
                        encrypted: false,
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
                let (metadata, is_onchain) = match connector {
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

                let ipfs_hash = ipfs_client.upload(metadata).await?;

                let id = match connector {
                    Connector::OnChain(on_chain) => &on_chain.id,
                    Connector::OffChain(off_chain) => &off_chain.id,
                };

                Ok::<ContractConnector, Box<dyn std::error::Error>>(ContractConnector {
                    id: id.clone(),
                    metadata: ipfs_hash,
                    encrypted: false,
                    onChain: is_onchain,
                })
            })
            .try_collect::<Vec<_>>()
            .await?,
            agents: stream::iter(&self.agents)
                .then(|agent| async {
                    let metadata = serde_json::to_vec(&agent.to_json())?;
                    let ipfs_hash = ipfs_client.upload(metadata).await?;
                    Ok::<ContractAgent, Box<dyn std::error::Error>>(ContractAgent {
                        id: agent.id().to_vec(),
                        metadata: ipfs_hash,
                        encrypted: false,
                        wallet: agent.wallet.address(),
                        writer: agent.write_role || agent.admin_role,
                    })
                })
                .try_collect::<Vec<_>>()
                .await?,
            evaluations: stream::iter(&self.evaluations)
                .then(|evaluation| async {
                    let metadata = serde_json::to_vec(&evaluation.to_json())?;
                    let ipfs_hash = ipfs_client.upload(metadata).await?;
                    Ok::<ContractEvaluation, Box<dyn std::error::Error>>(ContractEvaluation {
                        id: evaluation.id().to_vec(),
                        metadata: ipfs_hash,
                        encrypted: false,
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
    pub async fn persist_adapter(self) -> Result<(), Box<dyn Error>> {
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

                    let pending_tx = cliente.send_transaction(req, None).await.map_err(|e| {
                        eprintln!("Error sending the transaction: {:?}", e);
                        Box::<dyn std::error::Error>::from(format!(
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

        Ok(())
    }

    pub async fn remove_adapter(self) -> Result<(), Box<dyn Error>> {
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

                    let pending_tx = cliente.send_transaction(req, None).await.map_err(|e| {
                        eprintln!("Error sending the transaction: {:?}", e);
                        Box::<dyn std::error::Error>::from(format!(
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

        Ok(())
    }
}
