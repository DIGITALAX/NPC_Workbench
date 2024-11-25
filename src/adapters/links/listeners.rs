use crate::{nibble::Adaptable, utils::generate_unique_id};
use ethers::{
    abi::{decode, Abi, Address, RawLog, Token},
    contract::Contract,
    middleware::{Middleware, SignerMiddleware},
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    types::{Chain, Filter, Log, H160},
};
use reqwest::Client;
use serde_json::{from_slice, to_value, Map, Value};
use std::{error::Error, sync::Arc};
use tokio::{
    sync::mpsc::Sender,
    time::{sleep, Duration},
};

#[derive(Debug, Clone)]
pub struct Listener {
    pub name: String,
    pub id: Vec<u8>,
    pub listener_type: ListenerType,
    pub encrypted: bool,
}

#[derive(Debug, Clone)]
pub enum ListenerType {
    OnChain {
        contract_address: Address,
        event_signature: String,
        abi: String,
        provider: Provider<Http>,
        wallet: LocalWallet,
        chain: Chain,
    },
    OffChain {
        webhook_url: String,
        sns_verification: bool,
    },
    Timer {
        interval: Duration,
    },
}

pub fn configure_new_listener(
    name: &str,
    listener_type: ListenerType,
    encrypted: bool,
    address: &H160,
) -> Result<Listener, Box<dyn Error + Send + Sync>> {
    let listener = Listener {
        name: name.to_string(),
        id: generate_unique_id(address),
        listener_type,
        encrypted,
    };

    Ok(listener)
}

impl Adaptable for Listener {
    fn name(&self) -> &str {
        &self.name
    }
    fn id(&self) -> &Vec<u8> {
        &self.id
    }
}

impl Listener {
    pub fn to_json(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("name".to_string(), Value::String(self.name.clone()));
        map.insert("public".to_string(), Value::Bool(self.encrypted));

        let listener_type_map = match &self.listener_type {
            ListenerType::OnChain {
                contract_address,
                event_signature,
                abi,
                provider,
                wallet,
                chain,
            } => {
                let mut sub_map = Map::new();
                sub_map.insert(
                    "contract_address".to_string(),
                    Value::String(format!("{:?}", contract_address)),
                );
                sub_map.insert(
                    "event_signature".to_string(),
                    Value::String(event_signature.clone()),
                );
                sub_map.insert("abi".to_string(), Value::String(abi.clone()));
                sub_map.insert(
                    "provider".to_string(),
                    Value::String(format!("{:?}", provider)),
                );
                sub_map.insert("wallet".to_string(), Value::String(format!("{:?}", wallet)));
                sub_map.insert("chain".to_string(), Value::String(format!("{:?}", chain)));
                Value::Object(sub_map)
            }
            ListenerType::OffChain {
                webhook_url,
                sns_verification,
            } => {
                let mut sub_map = Map::new();

                sub_map.insert(
                    "sns_verification".to_string(),
                    Value::Bool(sns_verification.clone()),
                );
                sub_map.insert(
                    "webhook_url".to_string(),
                    Value::String(webhook_url.clone()),
                );

                Value::Object(sub_map)
            }
            ListenerType::Timer { interval } => {
                let mut sub_map = Map::new();
                sub_map.insert(
                    "interval".to_string(),
                    Value::String(format!("{:?}", interval)),
                );
                Value::Object(sub_map)
            }
        };
        map.insert("listener_type".to_string(), listener_type_map);

        map
    }

    pub async fn listen_and_trigger(
        &self,
        sender: Sender<Value>,
        repetitions: Option<u64>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut executed = 0;

        match &self.listener_type {
            ListenerType::OnChain {
                contract_address,
                event_signature,
                abi,
                provider,
                wallet,
                chain,
            } => {
                let client = SignerMiddleware::new(
                    provider.clone(),
                    wallet.clone().with_chain_id(chain.clone()),
                );
                let client = Arc::new(client);

                loop {
                    if let Some(max_reps) = repetitions {
                        if executed >= max_reps && max_reps > 0 {
                            println!("Max repetitions reached for OnChain listener.");
                            break;
                        }
                    }

                    let logs: Vec<Log> = client
                        .get_logs(
                            &Filter::new()
                                .address(*contract_address)
                                .event(event_signature),
                        )
                        .await?;

                    for log in logs {
                        println!("OnChain event detected: {:?}", log);
                        let decoded_event = decode_event(abi, &log, provider.clone())?;
                        sender.send(decoded_event).await?;
                    }

                    executed += 1;
                    sleep(Duration::from_secs(10)).await;
                }
            }

            ListenerType::OffChain {
                webhook_url,
                sns_verification,
            } => {
                let client = Client::new();

                loop {
                    if let Some(max_reps) = repetitions {
                        if executed >= max_reps && max_reps > 0 {
                            println!("Max repetitions reached for OffChain listener.");
                            break;
                        }
                    }

                    let response = client.get(webhook_url).send().await?;
                    let result = response.json::<Value>().await?;
                    if *sns_verification {
                        if let Some(payload_type) = result["Type"].as_str() {
                            match payload_type {
                                "SubscriptionConfirmation" => {
                                    if let Some(subscribe_url) = result["SubscribeURL"].as_str() {
                                        let confirm_response =
                                            client.get(subscribe_url).send().await?;
                                        if confirm_response.status().is_success() {
                                            println!("Subscription confirmed: {}", subscribe_url);
                                        } else {
                                            eprintln!(
                                                "Failed to confirm subscription: {}",
                                                subscribe_url
                                            );
                                        }
                                    }
                                }
                                "Notification" => {
                                    println!("SNS Notification received: {:?}", result);
                                    sender.send(result.clone()).await?;
                                }
                                "UnsubscribeConfirmation" => {
                                    println!("Received UnsubscribeConfirmation: {:?}", result);
                                }
                                _ => {
                                    println!("Unhandled SNS Type: {:?}", payload_type);
                                }
                            }
                        } else {
                            eprintln!("Invalid SNS payload: {:?}", result);
                        }
                    } else {
                        println!("Webhook data received: {:?}", result);
                        sender.send(result.clone()).await?;
                    }

                    executed += 1;
                    sleep(Duration::from_secs(5)).await;
                }
            }

            ListenerType::Timer { interval } => loop {
                if let Some(max_reps) = repetitions {
                    if executed >= max_reps && max_reps > 0 {
                        println!("Max repetitions reached for Timer listener.");
                        break;
                    }
                }

                sleep(*interval).await;

                let timer = Value::String(chrono::Utc::now().to_string());

                println!("Timer check completed at: {:?}", timer);
                sender.send(timer).await?;

                executed += 1;
            },
        }

        Ok(())
    }
}

fn decode_event(
    abi: &str,
    log: &Log,
    provider: Provider<Http>,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let abi: Abi = from_slice(abi.as_bytes())?;
    let contract = Contract::new(log.address, abi, Arc::new(provider));

    let event_signature = &log.topics[0];
    let event = contract
        .abi()
        .events()
        .find(|e| e.signature() == *event_signature)
        .ok_or("No matching event found in ABI")?
        .clone();

    let raw_log = RawLog {
        topics: log.topics.clone(),
        data: log.data.clone().to_vec(),
    };

    let decoded_event = decode(
        &event
            .inputs
            .iter()
            .map(|p| p.kind.clone())
            .collect::<Vec<_>>(),
        &raw_log.data,
    )?;

    let json_result: Value = to_value(
        decoded_event
            .into_iter()
            .map(|t| match t {
                Token::Uint(u) => Value::String(u.to_string()),
                Token::Address(a) => Value::String(format!("{:?}", a)),
                Token::String(s) => Value::String(s),
                Token::Bool(b) => Value::Bool(b),
                _ => Value::String(format!("{:?}", t)),
            })
            .collect::<Vec<Value>>(),
    )?;

    Ok(json_result)
}
