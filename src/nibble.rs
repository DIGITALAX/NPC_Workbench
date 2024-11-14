use crate::{
    adaptors::{
        agents::{self, Agent, LLMModel},
        conditions::{configure_new_condition, Condition, ConditionType},
        connectors::{
            offchain::{configure_new_offchain_connector, OffChainConnector},
            onchain::{configure_new_onchain_connector, OnChainConnector},
        },
        evaluations::{configure_new_evaluation, Evaluation, EvaluationType},
        fhegates::{configure_new_gate, FHEGate},
        listeners::{configure_new_listener, Listener, ListenerType},
    },
    constants::NIBBLE_FACTORY_CONTRACT,
};
use ethers::{
    abi::{Abi, AbiDecode},
    prelude::*,
    types::{Address, Eip1559TransactionRequest, NameOrAddress, U256},
};
use reqwest::Method;
use serde_json::Value;
use std::{collections::HashMap, error::Error, fs::File, io::Read, path::Path, sync::Arc};

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
    pub onchain_connectors: Vec<OnChainConnector>,
    pub offchain_connectors: Vec<OffChainConnector>,
    pub contracts: Vec<ContractInfo>,
    pub owner_wallet: LocalWallet,
    pub id: Option<Bytes>,
    pub count: U256,
}

impl Nibble {
    pub fn new(owner_private_key: &str) -> Result<Self, Box<dyn Error>> {
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
    ) -> Result<(), Box<dyn Error>> {
        configure_new_condition(
            self,
            name,
            condition_type,
            condition_fn,
            expected_value,
            public,
        )
    }

    pub fn add_fhe_gate(
        &mut self,
        name: &str,
        key: &str,
        public: bool,
    ) -> Result<(), Box<dyn Error>> {
        configure_new_gate(self, name, key, public)
    }

    pub fn add_evaluation(
        &mut self,
        name: &str,
        evaluation_type: EvaluationType,
        public: bool,
    ) -> Result<(), Box<dyn Error>> {
        configure_new_evaluation(self, name, evaluation_type, public)
    }

    pub fn add_onchain_connector(
        &mut self,
        name: &str,
        address: Address,
        public: bool,
    ) -> Result<(), Box<dyn Error>> {
        configure_new_onchain_connector(self, name, address, public)
    }

    pub fn add_offchain_connector(
        &mut self,
        name: &str,
        api_url: &str,
        public: bool,
        http_method: Method,
        headers: Option<HashMap<String, String>>,
        execution_fn: Option<Box<dyn Fn(Value) -> Result<Value, Box<dyn Error>> + Send + Sync>>,
    ) -> Result<(), Box<dyn Error>> {
        configure_new_offchain_connector(
            self,
            name,
            api_url,
            public,
            http_method,
            headers,
            execution_fn,
        )
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
        agents::configure_new_agent(
            self,
            name,
            role,
            personality,
            system,
            write_role,
            admin_role,
            token_role,
            model,
        )
    }

    pub async fn create_nibble(&mut self, chain: u64, rpc_url: &str) -> Result<(), Box<dyn Error>> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        let client =
            SignerMiddleware::new(provider, self.owner_wallet.clone().with_chain_id(chain));
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
            contract_instance.method::<_, ([Address; 7], Bytes, U256)>("deployFromFactory", {});

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
                        let return_values: ([Address; 7], Bytes, U256) =
                            <([Address; 7], Bytes, U256)>::decode(&log_data_bytes)?;

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

    pub fn persist_adaptors() {}
}

// #[derive(Debug)]
// pub struct Workflow {
//     pub id: String,
//     pub nibble_id: String,
//     pub steps: Vec<WorkflowStep>,
//     pub relations: Vec<WorkflowRelation>,
// }

// #[derive(Debug)]
// pub enum WorkflowRelation {
//     DependsOn(String),
//     Triggers(String),
//     RunsAfter(String),
// }

// #[derive(Debug)]
// pub enum WorkflowTrigger {
//     ConditionMet { condition_name: String },
//     EventTriggered { listener_name: String },
//     FHEGateOpen { gate_name: String },
//     ResponseContains { keyword: String },
//     Always,
// }

// #[derive(Debug)]
// pub enum WorkflowAction {
//     GenerateResponse {
//         agent_name: String,
//         prompt: String,
//     },
//     CallAPI {
//         connector_name: String,
//         params: serde_json::Value,
//     },
//     TriggerEvent {
//         event_name: String,
//         payload: serde_json::Value,
//     },
//     OnChainTransaction {
//         connector_name: String,
//         method: String,
//         params: serde_json::Value,
//     },
// }

// #[derive(Debug)]
// pub struct WorkflowStep {
//     pub action: WorkflowAction,
//     pub trigger: WorkflowTrigger,
// }

// impl Workflow {
//     pub fn new(nibble_id: &str) -> Result<Self, Box<dyn Error>> {
//         let id = Uuid::new_v4().to_string();
//         Ok(Self {
//             id,
//             nibble_id: nibble_id.to_string(),
//             steps: vec![],
//             relations: vec![],
//         })
//     }

//     pub fn add_step(&mut self, step: WorkflowStep) {
//         self.steps.push(step);
//     }
// }

// #[derive(Debug)]
// pub struct WorkflowController {
//     pub workflows: HashMap<String, Workflow>,
//     pub workflow_statuses: HashMap<String, WorkflowStatus>,
// }

// #[derive(Debug, PartialEq, Eq)]
// pub enum WorkflowStatus {
//     Pending,
//     Running,
//     Completed,
//     Paused,
//     WaitingOn(String),
// }

// impl WorkflowController {
//     pub fn new() -> Self {
//         WorkflowController {
//             workflows: HashMap::new(),
//             workflow_statuses: HashMap::new(),
//         }
//     }

//     pub fn register_workflow(&mut self, workflow: Workflow) {
//         self.workflow_statuses
//             .insert(workflow.id.clone(), WorkflowStatus::Pending);
//         self.workflows.insert(workflow.id.clone(), workflow);
//     }

//     pub async fn execute(
//         &mut self,
//         workflow_id: &str,
//         nibble: &mut Nibble,
//     ) -> Result<(), Box<dyn Error>> {
//         if let Some(workflow) = self.workflows.get(workflow_id) {
//             match self.workflow_statuses.get(workflow_id) {
//                 Some(WorkflowStatus::Pending) | Some(WorkflowStatus::Paused) => {
//                     self.workflow_statuses
//                         .insert(workflow_id.to_string(), WorkflowStatus::Running);
//                     for step in &workflow.steps {
//                         if workflow.check_step_trigger(&step.trigger, nibble, self)? {
//                             workflow.execute_step_action(&step.action, nibble).await?;
//                         }
//                     }
//                     self.workflow_statuses
//                         .insert(workflow_id.to_string(), WorkflowStatus::Completed);

//                     for relation in &workflow.relations {
//                         match relation {
//                             WorkflowRelation::Triggers(related_id) => {
//                                 self.workflow_statuses
//                                     .insert(related_id.clone(), WorkflowStatus::Pending);
//                             }
//                             _ => (),
//                         }
//                     }
//                 }
//                 Some(WorkflowStatus::WaitingOn(dep_id)) => {
//                     if let Some(status) = self.workflow_statuses.get(dep_id) {
//                         if *status == WorkflowStatus::Completed {
//                             self.workflow_statuses
//                                 .insert(workflow_id.to_string(), WorkflowStatus::Pending);
//                             self.execute(workflow_id, nibble).await?;
//                         }
//                     }
//                 }
//                 _ => (),
//             }
//         }
//         Ok(())
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     use std::{env::var, error::Error};

//     #[tokio::test]
//     async fn test_create_nibble() -> Result<(), Box<dyn Error>> {
//         dotenv().ok();
//         let owner_private_key = var("PRIVATE_KEY").unwrap();
//         let mut nibble = Nibble::new(&owner_private_key)?;

//         nibble.add_agent(
//             "Test Agent",
//             "admin",
//             "Estratega de memes",
//             "Sistema de IA",
//             true,
//             true,
//             true,
//             LLMModel::OpenAI {
//                 api_key: "fake_api_key".to_string(),
//                 model: "text-davinci-003".to_string(),
//                 temperature: 0.7,
//                 max_tokens: 200,
//                 top_p: 1.0,
//                 frequency_penalty: 0.0,
//                 presence_penalty: 0.0,
//                 system_prompt: None,
//             },
//         )?;

//         let token_name = "MemeToken";
//         let token_symbol = "MEME";
//         let chain_id = 80002;
//         let initial_supply = 1000000;
//         let rpc_url = "https://polygon-amoy.g.alchemy.com/v2/-c2wqcQaHvAc1u6emyOCwk_axYx_yFJ0";

//         nibble
//             .create_nibble(token_name, token_symbol, chain_id, initial_supply, rpc_url)
//             .await?;

//         Ok(())
//     }

//     #[test]
//     fn test_add_agent() -> Result<(), Box<dyn Error>> {
//         dotenv().ok();
//         let owner_private_key = var("PRIVATE_KEY").unwrap();
//         let mut nibble = Nibble::new(&owner_private_key)?;

//         nibble.add_agent(
//             "Test Agent",
//             "admin",
//             "Estratega de memes",
//             "Sistema de IA",
//             true,
//             false,
//             true,
//             LLMModel::OpenAI {
//                 api_key: "fake_api_key".to_string(),
//                 model: "text-davinci-003".to_string(),
//                 temperature: 0.7,
//                 max_tokens: 200,
//                 top_p: 1.0,
//                 frequency_penalty: 0.0,
//                 presence_penalty: 0.0,
//                 system_prompt: None,
//             },
//         )?;

//         assert_eq!(nibble.agents.len(), 1);
//         assert_eq!(nibble.agents[0].name, "Test Agent");
//         assert!(nibble.agents[0].write_role);
//         assert_eq!(nibble.agents[0].admin_role, false);
//         assert!(nibble.agents[0].token_role);

//         Ok(())
//     }
// }
