#[cfg(test)]
mod tests {
    use ethers::types::{Address, Chain, H160, U256};
    use npc_workbench::{
        adapters::{
            links::{
                evaluations::{EvaluationResponseType, EvaluationType},
                listeners::ListenerType,
            },
            nodes::{
                agents::{LLMModel, Objective},
                connectors::{off_chain::ConnectorType, on_chain::GasOptions},
            },
        },
        ipfs::IPFSProvider,
        nibble::Nibble,
    };
    use reqwest::Method;
    use serde_json::{json, to_string, Value};
    use std::{collections::HashMap, env, error::Error, str::FromStr, sync::Arc};
    use tokio::time::Duration;

    use dotenv::dotenv;

    #[tokio::test]
    async fn test_create_nibble() {
        dotenv().ok();
        let owner_private_key = env::var("PRIVATE_KEY").expect("API_KEY must be set in .env file");
        let rpc_url =  env::var("RPC").expect("API_KEY must be set in .env file");
        let ipfs_provider = IPFSProvider::Infura;
        let mut ipfs_config: HashMap<String, String> = HashMap::new();
        ipfs_config.insert("project_id".to_string(), "project-id".to_string());
        ipfs_config.insert(
            "project_secret".to_string(),
            "project-secret".to_string(),
        );
        let chain = Chain::PolygonAmoy;
        let graph_api_key = Some("graph-api-key".to_string());

        let new_nibble = Nibble::new(
            &owner_private_key,
            &rpc_url,
            ipfs_provider,
            ipfs_config,
            chain,
            graph_api_key,
            None,
        );

        match new_nibble {
            Ok(mut new) => {
                println!("Nibble initialized successfully");


                match new.create_nibble().await {
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
                        None
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

                // Subflow connected to Timer Listener for posting on Lens
                let off_chain_check_notifications = nibble.add_offchain_connector( "LensNotifications",
                ConnectorType::GraphQL {
                    query: r#"
                        query notifications($request: NotificationRequest) {
                            notifications(request: $request) {
                                items {
                                    ... on ReactionNotification {
                                        id
                                        reactions {
                                            profile {
                                                id
                                                name
                                                bio
                                            }
                                            reactions {
                                                reactionType
                                            }
                                        }
                                    }
                                    ... on CommentNotification {
                                        id
                                        comment {
                                            id
                                            createdAt
                                            content
                                            by {
                                                id
                                                name
                                            }
                                        }
                                    }
                                    ... on QuoteNotification {
                                        id
                                        quote {
                                            id
                                            createdAt
                                            content
                                            by {
                                                id
                                                name
                                            }
                                        }
                                    }
                                    ... on FollowNotification {
                                        id
                                        followers {
                                            id
                                            handle
                                            metadata
                                        }
                                    }  
                                    ... on MentionNotification {
                                        id
                                        publication {
                                           ... on Post {
                                                 id
                                                 createdAt
                                                 content
                                                 by {
                                                  id
                                                  name
                                                }
                                            }
                                            ... on Comment {
                                                 id
                                                 createdAt
                                                 content
                                                 by {
                                                  id
                                                  name
                                                }
                                            }
                                            ... on Quote {
                                                 id
                                                 createdAt
                                                 content
                                                 by {
                                                  id
                                                  name
                                                }
                                          }
                                        }
                                    }                                                              
                                }
                                pageInfo {
                                    next
                                    prev
                                }
                            }
                        }
                    "#.to_string(),
                    variables: Some(
                        [
                            ("request.profileId".to_string(), "{{profileId}}".to_string()),
                            ("request.limit".to_string(), "{{limit}}".to_string()),
                            ("request.cursor".to_string(), "{{cursor}}".to_string()),
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
                Some(Arc::new(|response: Value| -> Result<Value, Box<dyn Error + Send + Sync>> {
                    if let Some(data) = response["data"]["notifications"].as_object() {
                        Ok(Value::Object(data.clone()))
                    } else {
                        Err("Error processing notifications response".into())
                    }
                })),
                &H160::from_str("0x0000000000000000000000000000000000000001").unwrap(),None
            );

            let agent_notification_judge = nibble.add_evaluation("AgentEvaluationNotifications",  EvaluationType::AgentJudge {
                agent_id: "MemeMaster".into(),
                prompt: "Based on the notifications, decide which one I should respond to. In your response give me the entire object back of the chosen notification.".to_string(),
                response_type: EvaluationResponseType::Dynamic,
            }, false);

            // Then I would in the workflow use the meme master agent to right a response/reply "Then craft back the response in your role to increase the lore of the meme. Make your response in JSON format with a field of message and the response, and a field of id where you put the comment/quote ID of the message that the response is for. If I am replying to a follow by someone or creating a new publication/post then dont include anything in the field of id since the message will be a new publication.". 

            let lens_create_post_connector = nibble.add_offchain_connector(
                "LensCreatePost",
                ConnectorType::GraphQL {
                    query: r#"
                        mutation createOnchainPostTypedData($request: CreatePostTypedDataRequest!) {
                            createOnchainPostTypedData(request: $request) {
                                id
                                expiresAt
                                typedData {
                                    types {
                                        Post {
                                            name
                                            type
                                        }
                                    }
                                    domain {
                                        name
                                        chainId
                                        version
                                        verifyingContract
                                    }
                                    value {
                                        nonce
                                        deadline
                                        profileId
                                        contentURI
                                        actionModules
                                        actionModulesInitDatas
                                        referenceModule
                                        referenceModuleInitData
                                    }
                                }
                            }
                        }
                    "#.to_string(),
                    variables: Some(
                        [
                            ("request.contentURI".to_string(), "{{contentURI}}".to_string()),
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
                Some(Arc::new(|response: Value| -> Result<Value, Box<dyn Error + Send + Sync>> {
                    if let Some(data) = response["data"]["createOnchainPostTypedData"].as_object() {
                        Ok(Value::Object(data.clone()))
                    } else {
                        Err("Error processing createOnchainPostTypedData response".into())
                    }
                })),
                &H160::from_str("0x0000000000000000000000000000000000000001").unwrap(),None
            );
            

            let lens_broadcast_post_connector = nibble.add_offchain_connector(
                "LensBroadcastPost",
                ConnectorType::GraphQL {
                    query: r#"
                        mutation broadcastOnchain($request: BroadcastRequest!) {
                            broadcastOnchain(request: $request) {
                                ... on RelaySuccess {
                                    txHash
                                    txId
                                }
                                ... on RelayError {
                                    reason
                                }
                            }
                        }
                    "#.to_string(),
                    variables: Some(
                        [
                            ("request.id".to_string(), "{{id}}".to_string()),
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
                Some(Arc::new(|response: Value| -> Result<Value, Box<dyn Error + Send + Sync>> {
                    if let Some(relay_result) = response["data"]["broadcastOnchain"].as_object() {
                        if relay_result.contains_key("txHash") {
                            Ok(Value::String(
                                relay_result["txHash"]
                                    .as_str()
                                    .unwrap_or("Unknown Transaction Hash")
                                    .to_string(),
                            ))
                        } else if let Some(reason) = relay_result["reason"].as_str() {
                            Err(format!("Relay Error: {}", reason).into())
                        } else {
                            Err("Unexpected response format".into())
                        }
                    } else {
                        Err("Error processing broadcastOnchain response".into())
                    }
                })),
                &H160::from_str("0x0000000000000000000000000000000000000001").unwrap(),None
            );


            let lens_comment_onchain_connector = nibble.add_offchain_connector(
                "LensCommentOnchain",
                ConnectorType::GraphQL {
                    query: r#"
                        mutation commentOnchain($request: CommentOnchainRequest!) {
                            commentOnchain(request: $request) {
                                ... on RelaySuccess {
                                    txId
                                    txHash
                                }
                                ... on LensProfileManagerRelayError {
                                    reason
                                }
                            }
                        }
                    "#.to_string(),
                    variables: Some(
                        [
                            ("request.commentOn".to_string(), "{{publicationId}}".to_string()),
                            ("request.contentURI".to_string(), "{{metadataURI}}".to_string()),
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
                Some(Arc::new(|response: Value| -> Result<Value, Box<dyn Error + Send + Sync>> {
                    if let Some(result) = response["data"]["commentOnchain"].as_object() {
                        if let Some(tx_hash) = result["txHash"].as_str() {
                            Ok(Value::String(tx_hash.to_string()))
                        } else if let Some(reason) = result["reason"].as_str() {
                            Err(format!("Lens Comment Error: {}", reason).into())
                        } else {
                            Err("Unexpected response format".into())
                        }
                    } else {
                        Err("Error processing commentOnchain response".into())
                    }
                })),
                &H160::from_str("0x0000000000000000000000000000000000000001").unwrap(),None
            );
            

            let lens_quote_onchain_connector = nibble.add_offchain_connector(
                "LensQuoteOnchain",
                ConnectorType::GraphQL {
                    query: r#"
                        mutation quoteOnchain($request: QuoteOnchainRequest!) {
                            quoteOnchain(request: $request) {
                                ... on RelaySuccess {
                                    txId
                                    txHash
                                }
                                ... on LensProfileManagerRelayError {
                                    reason
                                }
                            }
                        }
                    "#.to_string(),
                    variables: Some(
                        [
                            ("request.quoteOn".to_string(), "{{publicationId}}".to_string()),
                            ("request.contentURI".to_string(), "{{metadataURI}}".to_string()),
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
                Some(Arc::new(|response: Value| -> Result<Value, Box<dyn Error + Send + Sync>> {
                    if let Some(result) = response["data"]["quoteOnchain"].as_object() {
                        if let Some(tx_hash) = result["txHash"].as_str() {
                            Ok(Value::String(tx_hash.to_string()))
                        } else if let Some(reason) = result["reason"].as_str() {
                            Err(format!("Lens Quote Error: {}", reason).into())
                        } else {
                            Err("Unexpected response format".into())
                        }
                    } else {
                        Err("Error processing quoteOnchain response".into())
                    }
                })),
                &H160::from_str("0x0000000000000000000000000000000000000001").unwrap(),None
            );

            // subflujo de crear el token
            // agent would generate lore / details of the token to send to the on-chain adapter
            let on_chain_adapter_create_memecoin = nibble.add_onchain_connector("CreateMemecoinConnector", None, false, Some(include_bytes!("../abis/NibbleFactory.json").into()), Some(serde_json::from_str(include_str!("../abis/NibbleFactory.json")).unwrap()), Chain::Polygon,  Some(GasOptions {
                max_fee_per_gas: Some(U256::from(1_000_000_000)), 
                max_priority_fee_per_gas: Some(U256::from(1_000_000)), 
                gas_limit: Some(U256::from(3_000_000)), 
                nonce: None,
            }));

            let on_chain_adapter_uniswap_pool = nibble.add_onchain_connector("CreateUniswapPoolConnector", Some("0x1F98431c8aD98523631AE4a59f267346ea31F984".parse::<Address>().unwrap()), false, None, Some(serde_json::from_str(include_str!("../abis/NibbleFactory.json")).unwrap()), Chain::Polygon,  Some(GasOptions {
                max_fee_per_gas: Some(U256::from(1_000_000_000)), 
                max_priority_fee_per_gas: Some(U256::from(1_000_000)), 
                gas_limit: Some(U256::from(3_000_000)), 
                nonce: None,
            }));

            let on_chain_adapter_balancer_pool = nibble.add_onchain_connector("CreateBalancerWeightedPool", Some("0x8e9aa87E45e92BAD84dE4fA65B9988F8235E15F8".parse::<Address>().unwrap()), false, None, Some(serde_json::from_str(include_str!("../abis/NibbleFactory.json")).unwrap()), Chain::Polygon,  Some(GasOptions {
                max_fee_per_gas: Some(U256::from(1_000_000_000)), 
                max_priority_fee_per_gas: Some(U256::from(1_000_000)), 
                gas_limit: Some(U256::from(3_000_000)), 
                nonce: None,
            }));
            

            /* 
            Workflow One:
             [1] Link timer 
             [2] Node off-chain to check lens notifications
             [3] Link evaluation to decide which notification to respond
             [4] Node agent to generate response
             [5] Node condition to choose off-chain connector type
             [6] Node off-chain connector to make lens interaction
            */

     
            /* 
            Workflow Two:
             [1] Node agent to generate token lore from agent
             [2] Node on-chain connector to deploy token contract
             [3] Node on-chain connector to create the uniswap pool
             [4] Node on-chain connector to create the balancer pool
            */


            /* 
            Workflow Three:
             [1] Link timer
             [2] Node off-chain connector to check notifications and interactions
             [3] Link evaluate to choose who to start distributing the token
             [4] Node on-chain connector to distribute the token to addresses
            */
            
            }
            Err(e) => {
                eprintln!("Error al crear objeto: {:?}", e);
                panic!("Test failed due a critical error during Nibble creation.");
            }
        }
            }
            Err(err) => {
                println!("Error with Nibble: {:?}", err);
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
