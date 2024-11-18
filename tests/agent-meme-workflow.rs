#[cfg(test)]
mod tests {
    use npc_workbench::{adapters::agents::LLMModel, ipfs::IPFSProvider, nibble::Nibble};
    use ethers::types::{ Chain, H160};
    use std::{collections::HashMap, str::FromStr};

    #[tokio::test]
    async fn test_create_nibble() {
        let owner_private_key = "0x<YOUR_PRIVATE_KEY>";
        let rpc_url = "https://rpc-url-for-chain";
        let ipfs_provider = IPFSProvider::Infura;
        let ipfs_config: HashMap<_, _> = HashMap::new();
        let chain = Chain::PolygonAmoy;
        let graph_api_key = Some("your-graph-api-key".to_string());

        let new_nibble = Nibble::new(
            owner_private_key,
            rpc_url,
            ipfs_provider,
            ipfs_config,
            chain,
            graph_api_key,
        );

        assert!(new_nibble.is_ok(), "Failed to create Nibble instance");

        if let Ok(mut nibble) = new_nibble {
            println!("Nibble created successfully with ID: {:?}", nibble.id);


        let agents = vec![
            (
                "MemeMaster",
                "Creative Storyteller",
                "Charismatic and witty, always looking to create viral-worthy content.",
                "Generate engaging and funny memes based on the lore of the token. Focus on creating content that resonates with the younger audience.",
                LLMModel::OpenAI {
                    api_key: "your-openai-api-key".to_string(),
                    model: "gpt-4".to_string(),
                    temperature: 0.7,
                    max_tokens: 200,
                    top_p: 0.9,
                    frequency_penalty: 0.1,
                    presence_penalty: 0.2,
                    system_prompt: Some("You are a master meme creator focusing on humor and cultural trends.".to_string()),
                },
                false, 
                false, 
                false, Some(Box::leak(Box::new(H160::from_str("0x1").unwrap())))
                , Some("farcaster1"), Some("lens1")
            ),
            (
                "AnalystAgent",
                "Data Analyst",
                "Methodical and detail-oriented, analyzing social media trends.",
                "Analyze social media trends and provide insights to optimize the reach and engagement of the token-related campaigns.",
                LLMModel::Claude {
                    api_key: "your-claude-api-key".to_string(),
                    model: "claude-2".to_string(),
                    temperature: 0.5,
                    max_tokens: 150,
                    top_k: Some(40),
                    top_p: 0.8,
                    system_prompt: Some("You are an analytical agent specializing in social media metrics and trend prediction.".to_string()),
                },
                false, 
                true,       false, Some(Box::leak(Box::new(H160::from_str("0x1").unwrap())))
                , Some("farcaster2"), Some("lens2")
            ),
            (
                "CommunityBuilder",
                "Engagement Specialist",
                "Friendly and approachable, fostering community growth.",
                "Engage with the community by responding to comments, initiating discussions, and encouraging token adoption.",
                LLMModel::Ollama {
                    api_key: "your-ollama-api-key".to_string(),
                    model: "ollama-chat".to_string(),
                    temperature: 0.8,
                    max_tokens: 250,
                    top_p: 0.85,
                    frequency_penalty: 0.1,
                    presence_penalty: 0.3,
                },
                true,  
                false,      false, None,None,None
            ),
        ];

        for (name, role, personality, system, model, write_role, admin_role, encrypted, wallet_address, farcaster, lens) in agents {
            let result = nibble.add_agent(
                name,
                role,
                personality,
                system,
                write_role,
                admin_role,
                model,
                encrypted, wallet_address.as_deref(), farcaster, lens

            );

            let error_message = format!("Failed to add agent: {}", name);
            assert!(result.is_ok(), "{}", error_message);
            
            println!("Agent {} added successfully.", name);
        }

        assert_eq!(nibble.agents.len(), 3, "Unexpected number of agents added");
        }


      
    }
}
