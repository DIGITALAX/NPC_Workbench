use crate::{
    nibble::{Adaptable, Nibble},
    utils::generate_unique_id,
};
use ethers::{
    abi::{AbiParser, Address, Token},
    types::{TransactionRequest, H160},
};
use serde_json::{from_value, Map, Value};
use std::{error::Error, str::FromStr};

#[derive(Debug, Clone)]
pub enum LogicalOperator {
    And,
    Or,
    Not,
}

impl FromStr for LogicalOperator {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "And" => Ok(LogicalOperator::And),
            "Or" => Ok(LogicalOperator::Or),
            "Not" => Ok(LogicalOperator::Not),
            _ => Err(format!("Invalid LogicalOperator: {}", s)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Condition {
    pub name: String,
    pub condition_type: ConditionType,
    pub check: ConditionCheck,
    pub encrypted: bool,
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
    ContextBased,
    TimeBased {
        comparison_time: chrono::NaiveTime,
        comparison_type: TimeComparisonType,
    },
    Composite {
        operator: LogicalOperator,
        sub_conditions: Vec<Condition>,
    },
}

impl ConditionType {
    pub fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        if let Some(on_chain) = value.get("OnChain") {
            let contract_address = on_chain
                .get("contract_address")
                .and_then(|v| v.as_str())
                .ok_or("Missing or invalid `contract_address`")?
                .parse::<Address>()
                .map_err(|_| "Invalid `contract_address`")?;

            let function_signature = on_chain
                .get("function_signature")
                .and_then(|v| v.as_str())
                .ok_or("Missing or invalid `function_signature`")?
                .to_string();

            Ok(ConditionType::OnChain {
                contract_address,
                function_signature,
            })
        } else if let Some(off_chain) = value.get("OffChain") {
            let api_url = off_chain
                .get("api_url")
                .and_then(|v| v.as_str())
                .ok_or("Missing or invalid `api_url`")?
                .to_string();

            Ok(ConditionType::OffChain { api_url })
        } else if let Some(time_based) = value.get("TimeBased") {
            let comparison_time = time_based
                .get("comparison_time")
                .and_then(|v| v.as_str())
                .ok_or("Missing or invalid `comparison_time`")?
                .parse::<chrono::NaiveTime>()
                .map_err(|_| "Invalid `comparison_time` format")?;

            let comparison_type = time_based
                .get("comparison_type")
                .and_then(|v| v.as_str())
                .ok_or("Missing or invalid `comparison_type`")?
                .parse::<TimeComparisonType>()?;

            Ok(ConditionType::TimeBased {
                comparison_time,
                comparison_type,
            })
        } else if let Some(composite) = value.get("Composite") {
            let operator = composite
                .get("operator")
                .and_then(|v| v.as_str())
                .ok_or("Missing or invalid `operator`")?
                .parse::<LogicalOperator>()?;

            let sub_conditions = composite
                .get("sub_conditions")
                .and_then(|v| v.as_array())
                .ok_or("Missing or invalid `sub_conditions`")?
                .iter()
                .map(Condition::from_json)
                .collect::<Result<Vec<Condition>, String>>()?;

            Ok(ConditionType::Composite {
                operator,
                sub_conditions,
            })
        } else {
            Err("Unknown `ConditionType` variant".to_string())
        }
    }
}

#[derive(Debug, Clone)]
pub enum TimeComparisonType {
    Before,
    After,
}

impl FromStr for TimeComparisonType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Before" => Ok(TimeComparisonType::Before),
            "After" => Ok(TimeComparisonType::After),
            _ => Err(format!("Invalid TimeComparisonType: {}", s)),
        }
    }
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

impl ConditionCheck {
    pub fn from_json(value: &Value) -> Result<Self, String> {
        let expected_value = value.get("expected_value").cloned();

        let condition_fn = |_value: Value| true;

        Ok(ConditionCheck {
            condition_fn,
            expected_value,
        })
    }
}

pub fn configure_new_condition(
    name: &str,
    condition_type: ConditionType,
    condition_fn: fn(Value) -> bool,
    expected_value: Option<Value>,
    encrypted: bool,
    address: &H160,
) -> Result<Condition, Box<dyn Error + Send + Sync>> {
    let check = ConditionCheck {
        condition_fn,
        expected_value,
    };

    let condition = Condition {
        name: name.to_string(),
        id: generate_unique_id(address),
        condition_type,
        check,
        encrypted,
    };

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
        map.insert("public".to_string(), Value::Bool(self.encrypted));

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
            ConditionType::ContextBased {} => {
                let sub_map = Map::new();

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
            ConditionType::Composite {
                operator,
                sub_conditions,
            } => {
                let mut sub_map = Map::new();

                sub_map.insert(
                    "operator".to_string(),
                    Value::String(format!("{:?}", operator)),
                );

                let sub_conditions_json: Vec<Value> = sub_conditions
                    .iter()
                    .map(|condition| {
                        let mut condition_map = Map::new();
                        condition_map.insert("condition".to_string(), condition.to_json().into());
                        Value::Object(condition_map)
                    })
                    .collect();

                sub_map.insert(
                    "sub_conditions".to_string(),
                    Value::Array(sub_conditions_json),
                );

                Value::Object(sub_map)
            }
        };
        map.insert("condition_type".to_string(), condition_type_map);

        let check_map = self.check.to_stringified();
        map.insert("check".to_string(), Value::Object(check_map));

        map
    }

    pub fn from_json(value: &Value) -> Result<Self, String> {
        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing or invalid `name`")?
            .to_string();

        let condition_type_value = value
            .get("condition_type")
            .ok_or("Missing `condition_type`")?;
        let condition_type = ConditionType::from_json(condition_type_value)?;

        let check_value = value.get("check").ok_or("Missing `check`")?;
        let check = ConditionCheck::from_json(check_value)?;

        let encrypted = value
            .get("encrypted")
            .and_then(|v| v.as_bool())
            .ok_or("Missing or invalid `encrypted`")?;

        let id = value
            .get("id")
            .and_then(|v| v.as_array())
            .ok_or("Missing or invalid `id`".to_string())?
            .iter()
            .map(|v| {
                v.as_u64()
                    .ok_or("Invalid `id` element".to_string())
                    .map(|n| n as u8)
            })
            .collect::<Result<Vec<u8>, String>>()?;

        Ok(Condition {
            name,
            condition_type,
            check,
            encrypted,
            id,
        })
    }

    pub async fn check_condition(
        &self,
        nibble_context: &Nibble,
        previous_node_result: Option<Value>,
        dynamic_params: Option<Value>,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        match &self.condition_type {
            ConditionType::OnChain {
                contract_address,
                function_signature,
            } => {
                let abi =
                    AbiParser::default().parse_str(&format!("function {};", function_signature))?;
                let func = abi.functions().next().ok_or("Function not found in ABI")?;

                let decoded_params: Vec<Token> = match dynamic_params {
                    Some(params) => params
                        .as_array()
                        .ok_or("dynamic_params must be an array")?
                        .iter()
                        .map(|param| from_value::<Token>(param.clone()))
                        .collect::<Result<Vec<_>, _>>()?,
                    None => vec![],
                };

                let call_data = func.encode_input(&decoded_params)?;
                let tx_request = TransactionRequest {
                    to: Some(contract_address.clone().into()),
                    data: Some(call_data.into()),
                    ..Default::default()
                };

                let call_result = nibble_context.provider.call_raw(&tx_request.into()).await?;
                let is_valid =
                    (self.check.condition_fn)(Value::String(format!("{:?}", call_result)));
                Ok(is_valid)
            }
            ConditionType::OffChain { api_url } => {
                let mut url = api_url.clone();
                if let Some(params) = &dynamic_params {
                    if let Some(map) = params.as_object() {
                        let query_string: Vec<String> = map
                            .iter()
                            .map(|(key, value)| format!("{}={}", key, value.as_str().unwrap_or("")))
                            .collect();
                        url = format!("{}?{}", api_url, query_string.join("&"));
                    }
                }

                let response = reqwest::get(&url).await?;
                let json: Value = response.json().await?;
                let is_valid = (self.check.condition_fn)(json);
                Ok(is_valid)
            }
            ConditionType::ContextBased {} => match previous_node_result {
                Some(context) => {
                    let is_valid = (self.check.condition_fn)(context);
                    Ok(is_valid)
                }
                None => {
                    Err("No context provided from the previous node to evaluate condition.".into())
                }
            },
            ConditionType::TimeBased {
                comparison_time,
                comparison_type,
            } => {
                let current_time = chrono::Local::now().time();
                let is_valid = match comparison_type {
                    TimeComparisonType::Before => current_time < *comparison_time,
                    TimeComparisonType::After => current_time > *comparison_time,
                };
                Ok(is_valid)
            }

            ConditionType::Composite {
                operator,
                sub_conditions,
            } => {
                self.handle_other_conditions(
                    nibble_context,
                    previous_node_result,
                    dynamic_params,
                    sub_conditions,
                    operator.clone(),
                )
                .await
            }
        }
    }

    async fn handle_other_conditions(
        &self,
        nibble_context: &Nibble,
        previous_node_result: Option<Value>,
        dynamic_params: Option<Value>,
        sub_conditions: &Vec<Condition>,
        operator: LogicalOperator,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let mut results = Vec::new();

        for sub_condition in sub_conditions {
            let is_valid = match &sub_condition.condition_type {
                ConditionType::OnChain {
                    contract_address,
                    function_signature,
                } => {
                    let abi = AbiParser::default()
                        .parse_str(&format!("function {};", function_signature))?;
                    let func = abi.functions().next().ok_or("Function not found in ABI")?;

                    let decoded_params: Vec<Token> = match &dynamic_params {
                        Some(params) => params
                            .as_array()
                            .ok_or("dynamic_params must be an array")?
                            .iter()
                            .map(|param| from_value::<Token>(param.clone()))
                            .collect::<Result<Vec<_>, _>>()?,
                        None => vec![],
                    };

                    let call_data = func.encode_input(&decoded_params)?;
                    let tx_request = TransactionRequest {
                        to: Some(contract_address.clone().into()),
                        data: Some(call_data.into()),
                        ..Default::default()
                    };

                    let call_result = nibble_context.provider.call_raw(&tx_request.into()).await?;
                    (sub_condition.check.condition_fn)(Value::String(format!("{:?}", call_result)))
                }
                ConditionType::OffChain { api_url } => {
                    let mut url = api_url.clone();
                    if let Some(params) = &dynamic_params {
                        if let Some(map) = params.as_object() {
                            let query_string: Vec<String> = map
                                .iter()
                                .map(|(key, value)| {
                                    format!("{}={}", key, value.as_str().unwrap_or(""))
                                })
                                .collect();
                            url = format!("{}?{}", api_url, query_string.join("&"));
                        }
                    }

                    let response = reqwest::get(&url).await?;
                    let json: Value = response.json().await?;
                    (sub_condition.check.condition_fn)(json)
                }
                ConditionType::ContextBased => {
                    if let Some(context) = &previous_node_result {
                        (sub_condition.check.condition_fn)(context.clone())
                    } else {
                        return Err("No context provided for ContextBased condition".into());
                    }
                }
                ConditionType::TimeBased {
                    comparison_time,
                    comparison_type,
                } => {
                    let current_time = chrono::Local::now().time();
                    match comparison_type {
                        TimeComparisonType::Before => current_time < *comparison_time,
                        TimeComparisonType::After => current_time > *comparison_time,
                    }
                }
                _ => return Err("Unsupported sub-condition type".into()),
            };

            results.push(is_valid);
        }

        let final_result = match operator {
            LogicalOperator::And => results.iter().all(|&res| res),
            LogicalOperator::Or => results.iter().any(|&res| res),
            LogicalOperator::Not => {
                if results.len() != 1 {
                    return Err("Not operator must have exactly one sub-condition".into());
                }
                !results[0]
            }
        };

        Ok(final_result)
    }
}
