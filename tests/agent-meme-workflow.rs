#[cfg(test)]
mod tests {
    use tokio::time::Duration;
    use ethers::types::{Chain, H160};
    use npc_workbench::{
        adapters::nodes::{
            agents::{LLMModel, Objective}, connectors::off_chain::ConnectorType, listeners::{ListenerType, OffChainCheck}
        },
        ipfs::IPFSProvider,
        nibble::Nibble,
    };
    use reqwest::Method;
    use serde_json::{json, to_string, Value};
    use std::{collections::HashMap, env, error::Error, str::FromStr, sync::Arc};

    use dotenv::dotenv;

    #[tokio::test]
    async fn test_create_nibble() {
        dotenv().ok();
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
            None,
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
                        LLMModel::Other {
                            url: "https://models.inference.ai.azure.com".to_string(),
    api_key: Some(env::var("AZURE_KEY").expect("API_KEY must be set in .env file")),
    result_path: "".to_string(),
    result_type: "".to_string(),
    body: vec![
        ("model".to_string(), "gpt-4o-mini".to_string()),
        ("temperature".to_string(), "1.0".to_string()),
        ("top_p".to_string(), "1.0".to_string()),
        (
            "messages".to_string(),
            to_string(&vec![
                json!({ "role": "system", "content": "You are a helpful assistant." }),
            ])
            .unwrap(),
        ),
    ]
    .into_iter()
    .collect(),
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
                            api_key: Some(env::var("CLAUDE_KEY").expect("API_KEY must be set in .env file")).unwrap(),
                            model: "claude-3-5-sonnet-latest".to_string(),
                            temperature: 0.5,
                            max_tokens: 150,
                            top_k: Some(40),
                            top_p: 0.8,
                            system_prompt: Some("You are an analytical agent specializing in social media metrics and trend prediction.".to_string()),
                            version:"2023-06-01".to_string(),
                            stop_sequences: None,
                            stream: false,
                            metadata: None,
                            tool_choice: None,
                            tools: None,
                       
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
                            model: "llama3.1:8b".to_string(),
                            temperature: 0.8,
                            max_tokens: 250,
                            top_p: 0.85,
                            frequency_penalty: 0.1,
                            presence_penalty: 0.3,
                            format: None,
                            suffix: None,
                            system: None,
                            template: None,
                            context: None,
                            stream: None,
                            raw: None,
                            keep_alive: None,
                            options: None,
                            images: None
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

                            // if name == "AnalystAgent" {

                            //     let result = agent
                            //     .adapter
                            //     .generate_objectives("Increase the type of memes that are created and make the lore about detailed.")
                            //     .await;

                            // assert!(result.is_ok(), "Objective generation failed");

                            // for objective in agent.adapter.objectives.iter() {
                            //     println!(
                            //         "Objective: {}, Priority: {}, Generated: {}",
                            //         objective.description,
                            //         objective.priority,
                            //         objective.generated
                            //     );
                            // }
                            // }

                            // if name == "CommunityBuilder" {
                            //     agent.adapter.add_objective(
                            //         "Collaborate with accounts on Lens that sell and create art NFTs that can be bought on Lens. Like and comment and repost these publications.",
                            //         7,
                            //         false,
                            //     );
                            //     let result = agent
                            //         .adapter
                            //         .generate_objectives("Interact with specific accounts on Lens of artists and creators and build the following and start interacting with them.")
                            //         .await;

                            //     assert!(result.is_ok(), "Objective generation failed");

                            //     for objective in agent.adapter.objectives.iter() {
                            //         println!(
                            //             "Objective: {}, Priority: {}, Generated: {}",
                            //             objective.description,
                            //             objective.priority,
                            //             objective.generated
                            //         );
                            //     }
                            // }
                        }
                        Err(e) => {
                            eprintln!("Error creating the agent: {:?}", e);
                            panic!("Test failed due a critical error during agent creation.");
                        }
                    }
                }

                assert_eq!(nibble.agents.len(), 3, "Unexpected number of agents added");
                let offchain_connectors = vec![
                    (
                        "LensRefreshToken",
                        ConnectorType::GraphQL {
                            query: r#"
                                mutation Refresh($request: RefreshRequest!) {
                                    refresh(request: $request) {
                                        accessToken
                                        refreshToken
                                        identityToken
                                    }
                                }
                            "#
                            .to_string(),
                            variables: Some(
                                [
                                    ("request.refreshToken".to_string(), "{{refreshToken}}".to_string()),
                                ]
                                .into_iter()
                                .collect(),
                            ),
                        },
                        "https://api-v2.lens.dev",
                        true,
                        Method::POST,
                        Some({
                            let mut headers = HashMap::new();
                            headers.insert("Content-Type".to_string(), "application/json".to_string());
                            headers.insert("Authorization".to_string(), "Bearer {{authToken}}".to_string());
                            headers
                        }),
                        None,
                        None,
                        "data",
                        "refresh",
                        H160::from_str("0x0000000000000000000000000000000000000001").unwrap(),
                    ),
                    (
                        "LensAuthenticate",
                        ConnectorType::GraphQL {
                            query: r#"
                                mutation Authenticate($request: SignedAuthChallenge!) {
                                    authenticate(request: $request) {
                                        accessToken
                                        identityToken
                                        refreshToken
                                    }
                                }
                            "#
                            .to_string(),
                            variables: Some(
                                [
                                    ("request.id".to_string(), "{{challengeId}}".to_string()),
                                    ("request.signature".to_string(), "{{signature}}".to_string()),
                                ]
                                .into_iter()
                                .collect(),
                            ),
                        },
                        "https://api-v2.lens.dev",
                        true,
                        Method::POST,
                        Some({
                            let mut headers = HashMap::new();
                            headers.insert("Content-Type".to_string(), "application/json".to_string());
                            headers
                        }),
                        None,
                        None,
                  
                        "data",
                        "authenticate",
                        H160::from_str("0x0000000000000000000000000000000000000001").unwrap(),
                    ),
                    (
                        "LensChallenge",
                        ConnectorType::GraphQL {
                            query: r#"
                                query Challenge($request: ChallengeRequest!) {
                                    challenge(request: $request) {
                                        id
                                        text
                                    }
                                }
                            "#
                            .to_string(),
                            variables: Some(
                                [
                                    ("request.signedBy".to_string(), "{{signedBy}}".to_string()),
                                    ("request.for".to_string(), "{{profileId}}".to_string()),
                                ]
                                .into_iter()
                                .collect(),
                            ),
                        },
                        "https://api-v2.lens.dev",
                        true,
                        Method::POST,
                        Some({
                            let mut headers = HashMap::new();
                            headers.insert("Content-Type".to_string(), "application/json".to_string());
                            headers
                        }),
                        None,
                        None,
                       "data",
                       "challenge",
                        H160::from_str("0x0000000000000000000000000000000000000001").unwrap(),
                    ),
                ];
                
                for (
                    name,
                    connector_type,
                    api_url,
                    encrypted,
                    http_method,
                    headers,
                    params,
                    auth_tokens,
                    value1,
                    value2,
                    address,
                ) in offchain_connectors
                {
                    let value1_cloned = value1.to_string();
                    let value2_cloned = value2.to_string();
                
                    let result = nibble.add_offchain_connector(
                        name,
                        connector_type,
                        api_url,
                        encrypted,
                        http_method,
                        headers,
                        params,
                        auth_tokens,
                        Some(Arc::new(move |response: Value| -> Result<Value, Box<dyn Error + Send + Sync>> {
                            if let Some(challenge) = response[&value1_cloned][&value2_cloned].as_object() {
                                Ok(Value::Object(challenge.clone()))
                            } else {
                                Err("Error processing challenge response".into())
                            }
                        })),
                        &address,
                    );
                
                    match result {
                        Ok(connector) => {
                            println!(
                                "Offchain Connector {} added successfully.",
                                connector.adapter.name
                            );
                    
                        }
                        Err(e) => {
                            eprintln!("Error creating the offchain connector: {:?}", e);
                            panic!("Test failed due a critical error during offchain connector creation.");
                        }
                    }
                }
                
                assert_eq!(
                    nibble.offchain_connectors.len(),
                    3,
                    "Unexpected number of offchain connectors added"
                );
                

                let interval = Duration::from_secs(1200);
                let listener_result = nibble.add_listener(
                    "Timer",
                    ListenerType::Timer {
                        interval,
                    },
                    false,
                );

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
[] subflujo de publicar
[] subflujo de balancer
[] subflujo de desplegar el token
[] alamcenar la información en cadena con el graph

*/
