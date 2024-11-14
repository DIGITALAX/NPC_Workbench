use std::error::Error;

use crate::nibble::Nibble;

#[derive(Debug)]
pub struct FHEGate {
    pub name: String,
    pub key: String,
    pub public: bool,
}

pub fn configure_new_gate(
    nibble: &mut Nibble,
    name: &str,
    key: &str,
    public: bool,
) -> Result<(), Box<dyn Error>> {
    nibble.fhe_gates.push(FHEGate {
        name: name.to_string(),
        key: key.to_string(),
        public,
    });
    Ok(())
}