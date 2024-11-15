use crate::{
    nibble::{Adaptable, Nibble},
    utils::generate_unique_id,
};
use ethers::abi::Address;
use serde_json::{Map, Value};
use std::error::Error;
use tokio::time::Duration;

use super::conditions::ConditionCheck;

#[derive(Debug, Clone)]
pub struct Listener {
    pub name: String,
    pub id: Vec<u8>,
    pub event_name: String,
    pub listener_type: ListenerType,
    pub condition: ConditionCheck,
    pub public: bool,
}

#[derive(Debug, Clone)]
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

pub fn configure_new_listener(
    nibble: &mut Nibble,
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

    nibble.listeners.push(Listener {
        name: name.to_string(),
        id: generate_unique_id(),
        event_name: event_name.to_string(),
        listener_type,
        condition,
        public,
    });

    Ok(())
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
        map.insert("public".to_string(), Value::Bool(self.public));

        let listener_type_map = match &self.listener_type {
            ListenerType::OnChain {
                contract_address,
                event_signature,
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
                Value::Object(sub_map)
            }
            ListenerType::OffChain { webhook_url } => {
                let mut sub_map = Map::new();
                sub_map.insert(
                    "webhook_url".to_string(),
                    Value::String(webhook_url.clone()),
                );
                Value::Object(sub_map)
            }
            ListenerType::Timer {
                interval,
                check_onchain,
                check_offchain,
            } => {
                let mut sub_map = Map::new();
                sub_map.insert(
                    "interval".to_string(),
                    Value::String(format!("{:?}", interval)),
                );
                sub_map.insert(
                    "check_onchain".to_string(),
                    match check_onchain {
                        Some(address) => Value::String(format!("{:?}", address)),
                        None => Value::Null,
                    },
                );
                sub_map.insert(
                    "check_offchain".to_string(),
                    match check_offchain {
                        Some(url) => Value::String(url.clone()),
                        None => Value::Null,
                    },
                );
                Value::Object(sub_map)
            }
        };
        map.insert("listener_type".to_string(), listener_type_map);

        let condition_map = self.condition.to_stringified();
        map.insert("condition".to_string(), Value::Object(condition_map));

        map
    }
}
