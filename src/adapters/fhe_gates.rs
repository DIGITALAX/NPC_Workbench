use std::error::Error;

use crate::{nibble::{Adaptable, Nibble}, utils::generate_unique_id};

#[derive(Debug, Clone)]
pub struct FHEGate {
    pub name: String,
    pub id: Vec<u8>,
    pub key: String,
    pub public: bool,
}

pub fn configure_new_gate(
    nibble: &mut Nibble,
    name: &str,
    key: &str,
    public: bool,
) -> Result<FHEGate, Box<dyn Error>> {
    let fhe_gate = FHEGate {
        name: name.to_string(),
        id: generate_unique_id(),
        key: key.to_string(),
        public,
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