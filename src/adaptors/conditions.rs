use crate::nibble::Nibble;
use ethers::abi::Address;
use serde_json::Value;
use std::error::Error;

#[derive(Debug)]
pub struct Condition {
    pub name: String,
    pub condition_type: ConditionType,
    pub check: ConditionCheck,
    pub public: bool,
}

#[derive(Debug)]
pub enum ConditionType {
    OnChain {
        contract_address: Address,
        function_signature: String,
    },
    OffChain {
        api_url: String,
    },
    InternalState {
        field_name: String,
    },
    ContextBased {
        key: String,
    },
    TimeBased {
        comparison_time: chrono::NaiveTime,
        comparison_type: TimeComparisonType,
    },
}

#[derive(Debug)]
pub enum TimeComparisonType {
    Before,
    After,
}

#[derive(Debug)]
pub struct ConditionCheck {
    pub condition_fn: fn(Value) -> bool,
    pub expected_value: Option<Value>,
}

pub fn configure_new_condition(
    nibble: &mut Nibble,
    name: &str,
    condition_type: ConditionType,
    condition_fn: fn(Value) -> bool,
    expected_value: Option<Value>,
    public: bool,
) -> Result<(), Box<dyn Error>> {
    let check = ConditionCheck {
        condition_fn,
        expected_value,
    };

    nibble.conditions.push(Condition {
        name: name.to_string(),
        condition_type,
        check,
        public,
    });

    Ok(())
}
