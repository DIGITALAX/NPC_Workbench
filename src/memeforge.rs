use ethers::{abi::Abi, core::rand::thread_rng, prelude::*, types::Address};
use std::{error::Error, fs::File, io::Read, path::Path, process::Command, sync::Arc};
use dotenv::dotenv;
use crate::utils::chain_to_network;

#[derive(Debug)]
pub struct ContractInfo {
    pub name: String,
    pub address: Address,
}

#[derive(Debug)]
pub struct Nibble {
    pub agents: Vec<Agent>,
    pub token: Option<Address>,
    pub contracts: Vec<ContractInfo>,
    pub owner_wallet: LocalWallet,
}
#[derive(Debug)]
pub struct MemeToken {
    pub name: String,
    pub symbol: String,
    pub chain: String,
    pub initial_supply: u64,
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
            "0x9A17abd59A716cD29bD1E56A33a2BC4780740AB4"
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
            .filter(|agent| !agent.write_role)
            .map(|agent| agent.wallet.address())
            .collect();

        let agent_admins: Vec<Address> = self
            .agents
            .iter()
            .filter(|agent| !agent.admin_role)
            .map(|agent| agent.wallet.address())
            .collect();

        let agent_tokens: Vec<Address> = self
            .agents
            .iter()
            .filter(|agent| !agent.token_role)
            .map(|agent| agent.wallet.address())
            .collect();

        let return_values: [Address; 4] = contract_instance
            .method::<_, [Address; 4]>(
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
            )?
            .call()
            .await?;

        self.contracts = vec![
            ContractInfo {
                name: "AccessControl".to_string(),
                address: return_values[0],
            },
            ContractInfo {
                name: "FHE".to_string(),
                address: return_values[1],
            },
            ContractInfo {
                name: "Data".to_string(),
                address: return_values[2],
            },
            ContractInfo {
                name: "Token".to_string(),
                address: return_values[3],
            },
        ];

        self.token = Some(return_values[3]);

        println!("AccessControl Contract: {:?}", return_values[0]);
        println!("FHE Contract: {:?}", return_values[1]);
        println!("Data Contract: {:?}", return_values[2]);
        println!("Token Contract: {:?}", return_values[3]);

        let output = Command::new("./deploy_subgraph.sh")
            .arg(name)
            .arg(format!("{}", return_values[0]))
            .arg(format!("{}", return_values[1]))
            .arg(format!("{}", return_values[2]))
            .arg(format!("{}", return_values[3]))
            .arg(chain_to_network(chain))
            .output()?;

        if !output.status.success() {
            eprintln!(
                "Error deploying the Subgraph: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return Err("Error executing the Subgraph deploy script.".into());
        }

        println!(
            "Subgraph deployed successfully: {}",
            String::from_utf8_lossy(&output.stdout)
        );

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
