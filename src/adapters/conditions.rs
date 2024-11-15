use crate::{
    nibble::{Adaptable, Nibble},
    utils::generate_unique_id,
};
use ethers::abi::Address;
use serde_json::{Map, Value};
use std::error::Error;

#[derive(Debug, Clone)]
pub struct Condition {
    pub name: String,
    pub condition_type: ConditionType,
    pub check: ConditionCheck,
    pub public: bool,
    pub id: Vec<u8>,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum TimeComparisonType {
    Before,
    After,
}

#[derive(Debug, Clone)]
pub struct ConditionCheck {
    pub condition_fn: fn(Value) -> bool,
    pub expected_value: Option<Value>,
}

impl ConditionCheck {
    pub fn to_stringified(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert(
            "condition_fn".to_string(),
            Value::String(format!("{:p}", self.condition_fn)),
        );
        map.insert(
            "expected_value".to_string(),
            match &self.expected_value {
                Some(value) => value.clone(),
                None => Value::Null,
            },
        );
        map
    }
}

pub fn configure_new_condition(
    nibble: &mut Nibble,
    name: &str,
    condition_type: ConditionType,
    condition_fn: fn(Value) -> bool,
    expected_value: Option<Value>,
    public: bool,
) -> Result<Condition, Box<dyn Error>> {
    let check = ConditionCheck {
        condition_fn,
        expected_value,
    };

    let condition = Condition {
        name: name.to_string(),
        id: generate_unique_id(),
        condition_type,
        check,
        public,
    };

    nibble.conditions.push(condition.clone());

    Ok(condition)
}

impl Adaptable for Condition {
    fn name(&self) -> &str {
        &self.name
    }
    fn id(&self) -> &Vec<u8> {
        &self.id
    }
}

impl Condition {
    pub fn to_json(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("name".to_string(), Value::String(self.name.clone()));
        map.insert("public".to_string(), Value::Bool(self.public));

        let condition_type_map = match &self.condition_type {
            ConditionType::OnChain {
                contract_address,
                function_signature,
            } => {
                let mut sub_map = Map::new();
                sub_map.insert(
                    "contract_address".to_string(),
                    Value::String(format!("{:?}", contract_address)),
                );
                sub_map.insert(
                    "function_signature".to_string(),
                    Value::String(function_signature.clone()),
                );
                Value::Object(sub_map)
            }
            ConditionType::OffChain { api_url } => {
                let mut sub_map = Map::new();
                sub_map.insert("api_url".to_string(), Value::String(api_url.clone()));
                Value::Object(sub_map)
            }
            ConditionType::InternalState { field_name } => {
                let mut sub_map = Map::new();
                sub_map.insert("field_name".to_string(), Value::String(field_name.clone()));
                Value::Object(sub_map)
            }
            ConditionType::ContextBased { key } => {
                let mut sub_map = Map::new();
                sub_map.insert("key".to_string(), Value::String(key.clone()));
                Value::Object(sub_map)
            }
            ConditionType::TimeBased {
                comparison_time,
                comparison_type,
            } => {
                let mut sub_map = Map::new();
                sub_map.insert(
                    "comparison_time".to_string(),
                    Value::String(comparison_time.format("%H:%M:%S").to_string()),
                );
                sub_map.insert(
                    "comparison_type".to_string(),
                    Value::String(format!("{:?}", comparison_type)),
                );
                Value::Object(sub_map)
            }
        };
        map.insert("condition_type".to_string(), condition_type_map);

        let check_map = self.check.to_stringified();
        map.insert("check".to_string(), Value::Object(check_map));

        map
    }
}
