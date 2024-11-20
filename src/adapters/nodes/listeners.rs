use crate::{nibble::Adaptable, utils::generate_unique_id};
use ethers::{
    abi::{decode, Abi, Address, RawLog, Token},
    contract::Contract,
    middleware::{Middleware, SignerMiddleware},
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    types::{Chain, Filter, Log, H160},
};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client,
};
use serde_json::{from_slice, from_value, to_value, Map, Value};
use std::{collections::HashMap, error::Error, sync::Arc};
use tokio::{
    sync::mpsc::Sender,
    time::{sleep, Duration},
};

#[derive(Debug, Clone)]
pub struct Listener {
    pub name: String,
    pub id: Vec<u8>,
    pub event_name: String,
    pub listener_type: ListenerType,
    pub encrypted: bool,
}

#[derive(Debug, Clone)]
pub enum ListenerType {
    OnChain {
        contract_address: Address,
        event_signature: String,
        abi: String,
        repetitions: Option<u64>,
        provider: Provider<Http>,
        wallet: LocalWallet,
        chain: Chain,
    },
    OffChain {
        webhook_url: String,
        repetitions: Option<u64>,
    },
    Timer {
        interval: Duration,
        check_onchain: Option<OnChainCheck>,
        check_offchain: Option<OffChainCheck>,
        repetitions: Option<u64>,
    },
}

#[derive(Debug, Clone)]
pub struct OnChainCheck {
    pub contract_address: Address,
    pub function_name: String,
    pub abi: String,
    pub args: Option<Value>,
    pub expected_return_type: String,
    pub provider: Provider<Http>,
    pub chain: Chain,
    pub wallet: LocalWallet,
}

#[derive(Debug, Clone)]
pub struct OffChainCheck {
    pub api_endpoint: String,
    pub params: Option<HashMap<String, String>>,
    pub headers: Option<HashMap<String, String>>,
    pub expected_return_type: String,
}

pub fn configure_new_listener(
    name: &str,
    event_name: &str,
    listener_type: ListenerType,
    encrypted: bool,
    address: &H160,
) -> Result<Listener, Box<dyn Error + Send + Sync>> {
    let listener = Listener {
        name: name.to_string(),
        id: generate_unique_id(address),
        event_name: event_name.to_string(),
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
        map.insert(
            "event_name".to_string(),
            Value::String(self.event_name.clone()),
        );
        map.insert("public".to_string(), Value::Bool(self.encrypted));

        let listener_type_map = match &self.listener_type {
            ListenerType::OnChain {
                contract_address,
                event_signature,
                abi,
                repetitions,
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
                    "repetitions".to_string(),
                    Value::Number(repetitions.map_or(serde_json::Number::from(0), |r| r.into())),
                );
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
                repetitions,
            } => {
                let mut sub_map = Map::new();
                sub_map.insert(
                    "webhook_url".to_string(),
                    Value::String(webhook_url.clone()),
                );
                sub_map.insert(
                    "repetitions".to_string(),
                    Value::Number(repetitions.map_or(serde_json::Number::from(0), |r| r.into())),
                );
                Value::Object(sub_map)
            }
            ListenerType::Timer {
                interval,
                check_onchain,
                check_offchain,
                repetitions,
            } => {
                let mut sub_map = Map::new();
                sub_map.insert(
                    "interval".to_string(),
                    Value::String(format!("{:?}", interval)),
                );
                sub_map.insert(
                    "check_onchain".to_string(),
                    match check_onchain {
                        Some(onchain_check) => {
                            let mut onchain_map = Map::new();
                            onchain_map.insert(
                                "contract_address".to_string(),
                                Value::String(format!("{:?}", onchain_check.contract_address)),
                            );
                            onchain_map.insert(
                                "function_name".to_string(),
                                Value::String(onchain_check.function_name.clone()),
                            );
                            onchain_map.insert(
                                "abi".to_string(),
                                Value::String(onchain_check.abi.clone()),
                            );
                            onchain_map.insert(
                                "args".to_string(),
                                onchain_check.args.clone().unwrap_or(Value::Null),
                            );
                            onchain_map.insert(
                                "expected_return_type".to_string(),
                                Value::String(onchain_check.expected_return_type.clone()),
                            );
                            onchain_map.insert(
                                "provider".to_string(),
                                Value::String(format!("{:?}", onchain_check.provider)),
                            );
                            onchain_map.insert(
                                "chain".to_string(),
                                Value::String(format!("{:?}", onchain_check.chain)),
                            );
                            onchain_map.insert(
                                "wallet".to_string(),
                                Value::String(format!("{:?}", onchain_check.wallet)),
                            );
                            Value::Object(onchain_map)
                        }
                        None => Value::Null,
                    },
                );
                sub_map.insert(
                    "check_offchain".to_string(),
                    match check_offchain {
                        Some(offchain_check) => {
                            let mut offchain_map = Map::new();
                            offchain_map.insert(
                                "api_endpoint".to_string(),
                                Value::String(offchain_check.api_endpoint.clone()),
                            );
                            offchain_map.insert(
                                "params".to_string(),
                                offchain_check
                                    .params
                                    .as_ref()
                                    .map(|p| {
                                        Value::Object(
                                            p.iter()
                                                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                                                .collect(),
                                        )
                                    })
                                    .unwrap_or(Value::Null),
                            );
                            offchain_map.insert(
                                "headers".to_string(),
                                offchain_check
                                    .headers
                                    .as_ref()
                                    .map(|h| {
                                        Value::Object(
                                            h.iter()
                                                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                                                .collect(),
                                        )
                                    })
                                    .unwrap_or(Value::Null),
                            );
                            offchain_map.insert(
                                "expected_return_type".to_string(),
                                Value::String(offchain_check.expected_return_type.clone()),
                            );
                            Value::Object(offchain_map)
                        }
                        None => Value::Null,
                    },
                );
                sub_map.insert(
                    "repetitions".to_string(),
                    Value::Number(repetitions.map_or(serde_json::Number::from(0), |r| r.into())),
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
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut executed = 0;

        match &self.listener_type {
            ListenerType::OnChain {
                contract_address,
                event_signature,
                abi,
                repetitions,
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
                        if executed >= *max_reps {
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
                repetitions,
            } => {
                let client = Client::new();

                loop {
                    if let Some(max_reps) = repetitions {
                        if executed >= *max_reps {
                            println!("Max repetitions reached for OffChain listener.");
                            break;
                        }
                    }

                    let response = client.get(webhook_url).send().await?;
                    let result = response.json::<Value>().await?;
                    println!("Webhook data received: {:?}", result);
                    sender.send(result).await?;

                    executed += 1;
                    sleep(Duration::from_secs(5)).await;
                }
            }

            ListenerType::Timer {
                interval,
                check_onchain,
                check_offchain,
                repetitions,
            } => loop {
                if let Some(max_reps) = repetitions {
                    if executed >= *max_reps {
                        println!("Max repetitions reached for Timer listener.");
                        break;
                    }
                }

                sleep(*interval).await;

                let result = match (check_onchain, check_offchain) {
                    (Some(onchain_check), _) => {
                        call_onchain_function(
                            &onchain_check.contract_address,
                            &onchain_check.function_name,
                            &onchain_check.abi,
                            onchain_check.args.clone(),
                            &onchain_check.expected_return_type,
                            &onchain_check.provider,
                        )
                        .await?
                    }

                    (_, Some(offchain_check)) => call_offchain_api(offchain_check).await?,

                    _ => Value::Null,
                };

                println!("Timer check completed: {:?}", result);
                sender.send(result).await?;

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

async fn call_onchain_function(
    contract_address: &Address,
    function_name: &str,
    abi: &str,
    args: Option<Value>,
    expected_return_type: &str,
    provider: &Provider<Http>,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let abi: Abi = from_slice(abi.as_bytes())?;
    let contract = Contract::new(*contract_address, abi, Arc::new(provider.clone()));
    let args: Vec<Token> = args.map_or(
        Ok::<Vec<Token>, Box<dyn Error + Send + Sync>>(vec![]),
        |a| from_value(a).map_err(|e| e.into()),
    )?;

    let call_result = contract
        .method::<Vec<Token>, _>(function_name, args)?
        .call()
        .await?;

    Ok(convert_to_expected_type(call_result, expected_return_type))
}

async fn call_offchain_api(
    offchain_check: &OffChainCheck,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let mut request = client.get(&offchain_check.api_endpoint);

    if let Some(params) = &offchain_check.params {
        request = request.query(params);
    }
    if let Some(headers) = &offchain_check.headers {
        let header_map = headers
            .iter()
            .filter_map(|(k, v)| {
                let name = k.parse::<HeaderName>().ok()?;
                let value = v.parse::<HeaderValue>().ok()?;
                Some((name, value))
            })
            .collect::<HeaderMap>();
        request = request.headers(header_map);
    }

    let response = request.send().await?;
    let result = if offchain_check.expected_return_type == "JSON" {
        response.json().await?
    } else {
        Value::String(response.text().await?)
    };

    Ok(result)
}

fn convert_to_expected_type(token: Token, expected_return_type: &str) -> Value {
    match expected_return_type {
        "uint256" => {
            let u256_value = token.into_uint().unwrap();
            Value::String(u256_value.to_string())
        }
        "string" => Value::String(token.into_string().unwrap()),
        "bool" => Value::Bool(token.into_bool().unwrap()),
        _ => Value::String(format!("{:?}", token)),
    }
}
