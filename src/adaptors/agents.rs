use crate::nibble::Nibble;
use ethers::{core::rand::thread_rng, prelude::*};
use std::error::Error;

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

pub fn configure_new_agent(
    nibble: &mut Nibble,
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

    nibble.agents.push(Agent {
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
