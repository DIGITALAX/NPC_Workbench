use ethers::types::U256;
use serde_json::Value;
use std::error::Error;

pub fn convert_value_to_token(value: &Value) -> Result<ethers::abi::Token, Box<dyn Error>> {
    match value {
        Value::Number(num) if num.is_u64() => Ok(ethers::abi::Token::Uint(U256::from(num.as_u64().unwrap()))),
        Value::String(s) => Ok(ethers::abi::Token::String(s.clone())),
        _ => Err("Unsupported parameter type".into()),
    }
}