use crate::nibble::Nibble;
use ethers::abi::Address;
use serde_json::Value;
use std::error::Error;
use tokio::time::Duration;

use super::conditions::ConditionCheck;

#[derive(Debug)]
pub struct Listener {
    pub name: String,
    pub event_name: String,
    pub listener_type: ListenerType,
    pub condition: ConditionCheck,
    pub public: bool,
}

#[derive(Debug)]
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
        event_name: event_name.to_string(),
        listener_type,
        condition,
        public,
    });

    Ok(())
}
