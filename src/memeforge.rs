use dotenv::dotenv;
use ethers::{
    abi::{Abi, AbiDecode},
    core::rand::thread_rng,
    prelude::*,
    types::{Address, Eip1559TransactionRequest, NameOrAddress, U256},
};
use reqwest::{Client, Method, RequestBuilder};
use serde_json::Value;
use std::{collections::HashMap, error::Error, fmt, fs::File, io::Read, path::Path, sync::Arc};
use tokio::time::Duration;
use uuid::Uuid;

use crate::utils::convert_value_to_token;

#[derive(Debug)]
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
    pub onchain_adapters: Vec<OnChainAdapter>,
    pub offchain_adapters: Vec<OffChainAdapter>,
    pub token: Option<MemeToken>,
    pub contracts: Vec<ContractInfo>,
    pub owner_wallet: LocalWallet,
    pub id: Option<String>,
    pub count: u64,
}
#[derive(Debug)]
pub struct MemeToken {
    pub name: String,
    pub symbol: String,
    pub chain: String,
    pub initial_supply: u64,
    pub address: Address,
}
#[derive(Debug)]
pub struct Agent {
    pub name: String,
    pub role: String,
    pub personality: String,
    pub system: String,
    pub model: LLMModel,
    pub wallet: LocalWallet,
    pub write_role: bool,
    pub admin_role: bool,
    pub token_role: bool,
}

#[derive(Debug)]
pub struct Condition {
    pub name: String,
    pub condition_type: ConditionType,
    pub check: ConditionCheck,
    pub public: bool,
}

#[derive(Debug)]
pub enum ConditionType {
    OnChain {
        contract_address: Address,
        function_signature: String,
    },
    OffChain {
        api_url: String,
    },
    InternalState {
        field_name: String,
    },
    ContextBased {
        key: String,
    },
    TimeBased {
        comparison_time: chrono::NaiveTime,
        comparison_type: TimeComparisonType,
    },
}

#[derive(Debug)]
pub enum TimeComparisonType {
    Before,
    After,
}

#[derive(Debug)]
pub struct Listener {
    pub name: String,
    pub event_name: String,
    pub listener_type: ListenerType,
    pub condition: ConditionCheck,
    pub public: bool,
}

#[derive(Debug)]
pub enum ListenerType {
    OnChain {
        contract_address: Address,
        event_signature: String,
    },
    OffChain {
        webhook_url: String,
    },
    Timer {
        interval: Duration,
        check_onchain: Option<Address>,
        check_offchain: Option<String>,
    },
}

#[derive(Debug)]
pub struct ConditionCheck {
    pub condition_fn: fn(Value) -> bool,
    pub expected_value: Option<Value>,
}

#[derive(Debug)]
pub struct FHEGate {
    pub name: String,
    pub key: String,
    pub public: bool,
}

#[derive(Debug)]
pub struct Evaluation {
    pub name: String,
    pub public: bool,
    pub evaluation_type: EvaluationType,
}

pub enum EvaluationType {
    HumanJudge {
        prompt: String,
        approval_required: bool,
    },
    LLMJudge {
        model_name: String,
        prompt_template: String,
        approval_threshold: f64,
    },
    ContextualJudge {
        context_fn: Arc<dyn Fn(Value) -> bool + Send + Sync>,
    },
}

impl std::fmt::Debug for EvaluationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvaluationType::HumanJudge {
                prompt,
                approval_required,
            } => f
                .debug_struct("HumanJudge")
                .field("prompt", prompt)
                .field("approval_required", approval_required)
                .finish(),
            EvaluationType::LLMJudge {
                model_name,
                prompt_template,
                approval_threshold,
            } => f
                .debug_struct("LLMJudge")
                .field("model_name", model_name)
                .field("prompt_template", prompt_template)
                .field("approval_threshold", approval_threshold)
                .finish(),
            EvaluationType::ContextualJudge { .. } => f
                .debug_struct("ContextualJudge")
                .field("context_fn", &"Function pointer")
                .finish(),
        }
    }
}

#[derive(Debug)]
pub struct OnChainAdapter {
    pub name: String,
    pub address: Address,
    pub public: bool,
    pub transactions: Vec<OnChainTransaction>,
}

#[derive(Debug)]
pub struct OnChainTransaction {
    pub function_signature: String,
    pub params: Vec<Value>,
    pub gas_options: GasOptions,
}

#[derive(Debug)]
pub struct GasOptions {
    pub max_fee_per_gas: Option<U256>,
    pub max_priority_fee_per_gas: Option<U256>,
    pub gas_limit: Option<U256>,
    pub nonce: Option<U256>,
}

impl Default for GasOptions {
    fn default() -> Self {
        GasOptions {
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            gas_limit: None,
            nonce: None,
        }
    }
}

pub struct OffChainAdapter {
    pub name: String,
    pub api_url: String,
    pub public: bool,
    pub http_method: Method,
    pub headers: Option<HashMap<String, String>>,
    pub execution_fn: Option<Box<dyn Fn(Value) -> Result<Value, Box<dyn Error>> + Send + Sync>>,
}

impl fmt::Debug for OffChainAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OffChainAdapter")
            .field("name", &self.name)
            .field("api_url", &self.api_url)
            .field("public", &self.public)
            .field("http_method", &self.http_method)
            .field("headers", &self.headers)
            .field("execution_fn", &"Function pointer")
            .finish()
    }
}

impl OffChainAdapter {
    pub fn new(
        name: &str,
        api_url: &str,
        public: bool,
        http_method: Method,
        headers: Option<HashMap<String, String>>,
    ) -> Self {
        OffChainAdapter {
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

impl Nibble {
    pub fn new(owner_private_key: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            token: None,
            agents: vec![],
            contracts: vec![],
            owner_wallet: owner_private_key.parse()?,
            id: None,
            count: 0,
            fhe_gates: vec![],
            evaluations: vec![],
            onchain_adapters: vec![],
            offchain_adapters: vec![],
            conditions: vec![],
            listeners: vec![],
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
        let condition = ConditionCheck {
            condition_fn,
            expected_value,
        };

        self.listeners.push(Listener {
            name: name.to_string(),
            event_name: event_name.to_string(),
            listener_type,
            condition,
            public,
        });

        Ok(())
    }

    pub fn add_condition(
        &mut self,
        name: &str,
        condition_type: ConditionType,
        condition_fn: fn(Value) -> bool,
        expected_value: Option<Value>,
        public: bool,
    ) -> Result<(), Box<dyn Error>> {
        let check = ConditionCheck {
            condition_fn,
            expected_value,
        };

        self.conditions.push(Condition {
            name: name.to_string(),
            condition_type,
            check,
            public,
        });

        Ok(())
    }

    pub fn add_fhe_gate(
        &mut self,
        name: &str,
        key: &str,
        public: bool,
    ) -> Result<(), Box<dyn Error>> {
        self.fhe_gates.push(FHEGate {
            name: name.to_string(),
            key: key.to_string(),
            public,
        });
        Ok(())
    }

    pub fn add_evaluation(
        &mut self,
        name: &str,
        evaluation_type: EvaluationType,
        public: bool,
    ) -> Result<(), Box<dyn Error>> {
        self.evaluations.push(Evaluation {
            name: name.to_string(),
            public,
            evaluation_type,
        });
        Ok(())
    }
    pub fn add_onchain_adapter(
        &mut self,
        name: &str,
        address: Address,
        public: bool,
    ) -> Result<(), Box<dyn Error>> {
        self.onchain_adapters.push(OnChainAdapter {
            name: name.to_string(),
            address,
            public,
            transactions: vec![],
        });
        Ok(())
    }

    pub fn add_offchain_adapter(
        &mut self,
        name: &str,
        api_url: &str,
        public: bool,
        http_method: Method,
        headers: Option<HashMap<String, String>>,
        execution_fn: Option<Box<dyn Fn(Value) -> Result<Value, Box<dyn Error>> + Send + Sync>>,
    ) -> Result<(), Box<dyn Error>> {
        self.offchain_adapters.push(OffChainAdapter {
            name: name.to_string(),
            api_url: api_url.to_string(),
            public,
            http_method,
            headers,
            execution_fn,
        });
        Ok(())
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
    ) -> Result<(), Box<dyn Error>> {
        let wallet = LocalWallet::new(&mut thread_rng());

        self.agents.push(Agent {
            name: name.to_string(),
            role: role.to_string(),
            personality: personality.to_string(),
            system: system.to_string(),
            model: model.clone(),
            write_role,
            admin_role,
            token_role,
            wallet: wallet.clone(),
        });

        println!(
            "Agent Created: {} - Role: {} - Model: {:?} - Address {:?}",
            name,
            role,
            model,
            wallet.address(),
        );

        Ok(())
    }

    pub async fn create_nibble(
        &mut self,
        name: &str,
        symbol: &str,
        chain: u64,
        initial_supply: u64,
        rpc_url: &str,
    ) -> Result<(), Box<dyn Error>> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        let client =
            SignerMiddleware::new(provider, self.owner_wallet.clone().with_chain_id(chain));
        let client = Arc::new(client);

        let mut abi_file = File::open(Path::new("./abis/AgentMemeFactory.json"))?;
        let mut abi_content = String::new();
        abi_file.read_to_string(&mut abi_content)?;
        let abi = serde_json::from_str::<Abi>(&abi_content)?;

        let contract_instance = Contract::new(
            "0x7B2E5faAEf3715D6603DA889cfac81b010aab36F"
                .parse::<Address>()
                .unwrap(),
            abi,
            client.clone(),
        );

        let agent_writers: Vec<Address> = self
            .agents
            .iter()
            .filter(|agent| agent.write_role)
            .map(|agent| agent.wallet.address())
            .collect();

        let agent_readers: Vec<Address> = self
            .agents
            .iter()
            .filter(|agent| agent.write_role == false)
            .map(|agent| agent.wallet.address())
            .collect();

        let agent_admins: Vec<Address> = self
            .agents
            .iter()
            .filter(|agent| agent.admin_role == true)
            .map(|agent| agent.wallet.address())
            .collect();

        let agent_tokens: Vec<Address> = self
            .agents
            .iter()
            .filter(|agent| agent.token_role == true)
            .map(|agent| agent.wallet.address())
            .collect();

        let method = contract_instance.method::<_, [Address; 4]>(
            "deployFromFactory",
            (
                agent_writers,
                agent_readers,
                agent_admins,
                agent_tokens,
                name.to_string(),
                symbol.to_string(),
                initial_supply,
            ),
        );

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
                            "0x7B2E5faAEf3715D6603DA889cfac81b010aab36F"
                                .parse::<Address>()
                                .unwrap(),
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
                        let return_values: ([Address; 4], String, u64) =
                            <([Address; 4], String, u64)>::decode(&log_data_bytes)?;

                        self.contracts = vec![
                            ContractInfo {
                                name: "AccessControl".to_string(),
                                address: return_values.0[0],
                            },
                            ContractInfo {
                                name: "FHE".to_string(),
                                address: return_values.0[1],
                            },
                            ContractInfo {
                                name: "Data".to_string(),
                                address: return_values.0[2],
                            },
                            ContractInfo {
                                name: "Token".to_string(),
                                address: return_values.0[3],
                            },
                        ];
                        self.id = Some(return_values.1);
                        self.count = return_values.2;
                        self.token = Some(MemeToken {
                            address: return_values.0[3],
                            name: name.to_string(),
                            symbol: symbol.to_string(),
                            chain: chain.to_string(),
                            initial_supply,
                        });

                        println!("AccessControl Contract: {:?}", return_values.0[0]);
                        println!("FHE Contract: {:?}", return_values.0[1]);
                        println!("Data Contract: {:?}", return_values.0[2]);
                        println!("Token Contract: {:?}", return_values.0[3]);

                        Ok(())
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
}

impl OnChainAdapter {
    pub fn add_transaction(
        &mut self,
        function_signature: &str,
        params: Vec<Value>,
        gas_options: GasOptions,
    ) -> Result<(), Box<dyn Error>> {
        self.transactions.push(OnChainTransaction {
            function_signature: function_signature.to_string(),
            params,
            gas_options,
        });
        Ok(())
    }
    pub async fn execute_transactions(
        &self,
        client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    ) -> Result<(), Box<dyn Error>> {
        for tx in &self.transactions {
            let encoded_data = self.encode_function_call(&tx.function_signature, &tx.params)?;

            let mut tx_request = Eip1559TransactionRequest::new()
                .to(NameOrAddress::Address(self.address))
                .data(encoded_data);

            if let Some(gas_limit) = tx.gas_options.gas_limit {
                tx_request = tx_request.gas(gas_limit);
            }
            if let Some(max_fee) = tx.gas_options.max_fee_per_gas {
                tx_request = tx_request.max_fee_per_gas(max_fee);
            }
            if let Some(priority_fee) = tx.gas_options.max_priority_fee_per_gas {
                tx_request = tx_request.max_priority_fee_per_gas(priority_fee);
            }
            if let Some(nonce) = tx.gas_options.nonce {
                tx_request = tx_request.nonce(nonce);
            }

            let pending_tx = client.send_transaction(tx_request, None).await?;
            let receipt = pending_tx.await?;

            match receipt {
                Some(r) => println!("Transaction executed with status: {:?}", r.status),
                None => println!("Transaction was not mined"),
            }
        }
        Ok(())
    }

    fn encode_function_call(
        &self,
        function_signature: &str,
        params: &[Value],
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let abi =
            abi::AbiParser::default().parse_str(&format!("function {};", function_signature))?;
        let func = abi.functions().next().ok_or("Function not found")?;

        let tokens = params
            .iter()
            .map(|p| convert_value_to_token(p))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(func.encode_input(&tokens)?)
    }
}

#[derive(Debug)]
pub struct Workflow {
    pub id: String,
    pub nibble_id: String,
    pub steps: Vec<WorkflowStep>,
    pub relations: Vec<WorkflowRelation>,
}

#[derive(Debug)]
pub enum WorkflowRelation {
    DependsOn(String),
    Triggers(String),
    RunsAfter(String),
}

#[derive(Debug)]
pub enum WorkflowTrigger {
    ConditionMet { condition_name: String },
    EventTriggered { listener_name: String },
    FHEGateOpen { gate_name: String },
    ResponseContains { keyword: String },
    Always,
}

#[derive(Debug)]
pub enum WorkflowAction {
    GenerateResponse {
        agent_name: String,
        prompt: String,
    },
    CallAPI {
        adapter_name: String,
        params: serde_json::Value,
    },
    TriggerEvent {
        event_name: String,
        payload: serde_json::Value,
    },
    OnChainTransaction {
        adapter_name: String,
        method: String,
        params: serde_json::Value,
    },
}

#[derive(Debug)]
pub struct WorkflowStep {
    pub action: WorkflowAction,
    pub trigger: WorkflowTrigger,
}

impl Workflow {
    pub fn new(nibble_id: &str) -> Result<Self, Box<dyn Error>> {
        let id = Uuid::new_v4().to_string();
        Ok(Self {
            id,
            nibble_id: nibble_id.to_string(),
            steps: vec![],
            relations: vec![],
        })
    }

    pub fn add_step(&mut self, step: WorkflowStep) {
        self.steps.push(step);
    }
}

#[derive(Debug)]
pub struct WorkflowController {
    pub workflows: HashMap<String, Workflow>,
    pub workflow_statuses: HashMap<String, WorkflowStatus>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum WorkflowStatus {
    Pending,
    Running,
    Completed,
    Paused,
    WaitingOn(String),
}

impl WorkflowController {
    pub fn new() -> Self {
        WorkflowController {
            workflows: HashMap::new(),
            workflow_statuses: HashMap::new(),
        }
    }

    pub fn register_workflow(&mut self, workflow: Workflow) {
        self.workflow_statuses
            .insert(workflow.id.clone(), WorkflowStatus::Pending);
        self.workflows.insert(workflow.id.clone(), workflow);
    }

    pub async fn execute(
        &mut self,
        workflow_id: &str,
        nibble: &mut Nibble,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(workflow) = self.workflows.get(workflow_id) {
            match self.workflow_statuses.get(workflow_id) {
                Some(WorkflowStatus::Pending) | Some(WorkflowStatus::Paused) => {
                    self.workflow_statuses
                        .insert(workflow_id.to_string(), WorkflowStatus::Running);
                    for step in &workflow.steps {
                        if workflow.check_step_trigger(&step.trigger, nibble, self)? {
                            workflow.execute_step_action(&step.action, nibble).await?;
                        }
                    }
                    self.workflow_statuses
                        .insert(workflow_id.to_string(), WorkflowStatus::Completed);

                    for relation in &workflow.relations {
                        match relation {
                            WorkflowRelation::Triggers(related_id) => {
                                self.workflow_statuses
                                    .insert(related_id.clone(), WorkflowStatus::Pending);
                            }
                            _ => (),
                        }
                    }
                }
                Some(WorkflowStatus::WaitingOn(dep_id)) => {
                    if let Some(status) = self.workflow_statuses.get(dep_id) {
                        if *status == WorkflowStatus::Completed {
                            self.workflow_statuses
                                .insert(workflow_id.to_string(), WorkflowStatus::Pending);
                            self.execute(workflow_id, nibble).await?;
                        }
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{env::var, error::Error};

    #[tokio::test]
    async fn test_create_nibble() -> Result<(), Box<dyn Error>> {
        dotenv().ok();
        let owner_private_key = var("PRIVATE_KEY").unwrap();
        let mut nibble = Nibble::new(&owner_private_key)?;

        nibble.add_agent(
            "Test Agent",
            "admin",
            "Estratega de memes",
            "Sistema de IA",
            true,
            true,
            true,
            LLMModel::OpenAI {
                api_key: "fake_api_key".to_string(),
                model: "text-davinci-003".to_string(),
                temperature: 0.7,
                max_tokens: 200,
                top_p: 1.0,
                frequency_penalty: 0.0,
                presence_penalty: 0.0,
                system_prompt: None,
            },
        )?;

        let token_name = "MemeToken";
        let token_symbol = "MEME";
        let chain_id = 80002;
        let initial_supply = 1000000;
        let rpc_url = "https://polygon-amoy.g.alchemy.com/v2/-c2wqcQaHvAc1u6emyOCwk_axYx_yFJ0";

        nibble
            .create_nibble(token_name, token_symbol, chain_id, initial_supply, rpc_url)
            .await?;

        Ok(())
    }

    #[test]
    fn test_add_agent() -> Result<(), Box<dyn Error>> {
        dotenv().ok();
        let owner_private_key = var("PRIVATE_KEY").unwrap();
        let mut nibble = Nibble::new(&owner_private_key)?;

        nibble.add_agent(
            "Test Agent",
            "admin",
            "Estratega de memes",
            "Sistema de IA",
            true,
            false,
            true,
            LLMModel::OpenAI {
                api_key: "fake_api_key".to_string(),
                model: "text-davinci-003".to_string(),
                temperature: 0.7,
                max_tokens: 200,
                top_p: 1.0,
                frequency_penalty: 0.0,
                presence_penalty: 0.0,
                system_prompt: None,
            },
        )?;

        assert_eq!(nibble.agents.len(), 1);
        assert_eq!(nibble.agents[0].name, "Test Agent");
        assert!(nibble.agents[0].write_role);
        assert_eq!(nibble.agents[0].admin_role, false);
        assert!(nibble.agents[0].token_role);

        Ok(())
    }
}
