#[cfg(test)]
mod tests {
    use ethers::types::{Chain, H160};
    use npc_workbench::{
        adapters::nodes::{
            agents::{LLMModel, Objective},
            listeners::{ListenerType, OffChainCheck},
        },
        ipfs::IPFSProvider,
        nibble::Nibble,
    };
    use std::{collections::HashMap, str::FromStr};
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_create_nibble() {
        let owner_private_key =
            "0x4c0883a69102937d6231471b5dbb6204fe512961708279385fe482433a9d3fb9";
        let rpc_url = "https://rpc-url-for-chain";
        let ipfs_provider = IPFSProvider::Infura;
        let mut ipfs_config: HashMap<String, String> = HashMap::new();
        ipfs_config.insert("project_id".to_string(), "your-project-id".to_string());
        ipfs_config.insert(
            "project_secret".to_string(),
            "your-project-secret".to_string(),
        );
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

        match new_nibble {
            Ok(mut nibble) => {
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
                        false,Some(Box::leak(Box::new(H160::from_str("0x0000000000000000000000000000000000000001").unwrap())))

                        , Some("farcaster1"), Some("lens1"),  vec![Objective {
                            description: "Build an initial audience".to_string(),
                            priority: 8,
                            generated: false,
                        }],
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
                        true,       false, Some(Box::leak(Box::new(H160::from_str("0x0000000000000000000000000000000000000002").unwrap())))

                        , Some("farcaster2"), Some("lens2"), vec![]
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
                        false,      false, None,None,None, vec![]
                    ),
                ];

                for (
                    name,
                    role,
                    personality,
                    system,
                    model,
                    write_role,
                    admin_role,
                    encrypted,
                    wallet_address,
                    farcaster,
                    lens,
                    objectives,
                ) in agents
                {
                    let result = nibble.add_agent(
                        name,
                        role,
                        personality,
                        system,
                        write_role,
                        admin_role,
                        model,
                        encrypted,
                        wallet_address.as_deref(),
                        farcaster,
                        lens,
                        objectives,
                    );

                    match result {
                        Ok(mut agent) => {
                            println!("Agent {} added successfully.", name);

                            if name == "CommunityBuilder" {
                                agent.adapter.add_objective(
                                    "Collaborate with influencers",
                                    7,
                                    false,
                                );
                                let result = agent
                                    .adapter
                                    .generate_objectives("The meme campaign needs more visibility")
                                    .await;

                                assert!(result.is_ok(), "Objective generation failed");

                                for objective in agent.adapter.objectives.iter() {
                                    println!(
                                        "Objective: {}, Priority: {}, Generated: {}",
                                        objective.description,
                                        objective.priority,
                                        objective.generated
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error al crear agente: {:?}", e);
                            panic!("Test failed due a critical error during agent creation.");
                        }
                    }
                }

                assert_eq!(nibble.agents.len(), 3, "Unexpected number of agents added");

                let api_url = "https://api.lens.dev/notifications";
                let interval = Duration::from_secs(1200);
                let listener_result = nibble.add_listener(
                    "LensApiMonitor",
                    "Lens Notifications",
                    ListenerType::Timer {
                        interval,
                        check_onchain: None,
                        check_offchain: Some(OffChainCheck {
                            api_endpoint: api_url.to_string(),
                            params: None,
                            headers: None,
                            expected_return_type: "JSON".to_string(),
                        }),
                        repetitions: Some(3),
                    },
                    false,
                );

                match listener_result {
                    Ok(_) => println!("Listener for Lens API added successfully."),
                    Err(e) => {
                        eprintln!("Failed to add Lens API Listener: {:?}", e);
                        panic!("Test failed due to listener creation error.");
                    }
                }


            }
            Err(e) => {
                eprintln!("Error al crear objeto: {:?}", e);
                panic!("Test failed due a critical error during Nibble creation.");
            }
        }




    }
}

/*

[x] Create agents with objectives and personalities
[] Create listeners and links to post and interact continuously on-chain
[] añade las puertas

1. Network Agents
Define distinct agents with specific personalities and roles.
Link these agents to their social media accounts via Lens Protocol and Farcaster.
Each agent focuses on a particular aspect of the lore aligned with their personality and specific objectives.
These agents operate continuously on the server, posting periodically according to their timers, while also interacting and responding to other accounts on social media based on predefined rules.
2. Token Deployment
When certain conditions are met, a process is triggered to deploy the meme token on-chain, using characteristics predefined by the user or specified by an agent (such as name, symbol, maximum supply, monthly distribution caps, or weekly distribution goals).
3. Infrastructure Creation for the Token
An initial token supply is minted to establish liquidity pools on Uniswap and Balancer.
4. Agents Receive Token Information
Network agents are updated with details about the token, its official liquidity pools, and other relevant information. This is added to their system prompt, enabling them to:
Post about the token.
Begin their second task, which involves distributing the token, conducting airdrops, and carrying out related activities according to their roles and the conditions governing these actions.
5. Expanding Agent Capabilities
Agents can later trigger additional workflows, such as initiating airdrops or distributing tokens as commissions for developers and artists. This feature can be developed in subsequent phases. Engage in trading strategies to build out the meme coin treasury.
*/
