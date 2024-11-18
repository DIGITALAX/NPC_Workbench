use std::error::Error;

use ethers::types::H160;

use crate::{
    nibble::{Adaptable, Nibble},
    utils::generate_unique_id,
};

#[derive(Debug, Clone)]
pub struct FHEGate {
    pub name: String,
    pub id: Vec<u8>,
    pub key: String,
    pub encrypted: bool,
}

pub fn configure_new_gate(
    nibble: &mut Nibble,
    name: &str,
    key: &str,
    encrypted: bool,
    address: &H160,
) -> Result<FHEGate, Box<dyn Error + Send + Sync>> {
    let fhe_gate = FHEGate {
        name: name.to_string(),
        id: generate_unique_id(address),
        key: key.to_string(),
        encrypted,
    };
    nibble.fhe_gates.push(fhe_gate.clone());
    Ok(fhe_gate)
}

impl Adaptable for FHEGate {
    fn name(&self) -> &str {
        &self.name
    }
    fn id(&self) -> &Vec<u8> {
        &self.id
    }
}

impl FHEGate {
    pub async fn check_fhe_gate(&self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        Ok(true)
    }
}
