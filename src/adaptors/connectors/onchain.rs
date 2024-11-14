use crate::{nibble::Nibble, utils::convert_value_to_token};
use ethers::{
    abi,
    prelude::*,
    types::{Address, Eip1559TransactionRequest, NameOrAddress, U256},
};
use serde_json::Value;
use std::{error::Error, sync::Arc};

#[derive(Debug)]
pub struct OnChainConnector {
    pub name: String,
    pub address: Address,
    pub public: bool,
    pub transactions: Vec<OnChainTransaction>,
}

#[derive(Debug)]
pub struct OnChainTransaction {
    pub function_signature: String,
    pub params: Vec<Value>,
    pub gas_options: GasOptions,
}

#[derive(Debug)]
pub struct GasOptions {
    pub max_fee_per_gas: Option<U256>,
    pub max_priority_fee_per_gas: Option<U256>,
    pub gas_limit: Option<U256>,
    pub nonce: Option<U256>,
}

impl Default for GasOptions {
    fn default() -> Self {
        GasOptions {
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            gas_limit: None,
            nonce: None,
        }
    }
}

pub fn configure_new_onchain_connector(
    nibble: &mut Nibble,
    name: &str,
    address: Address,
    public: bool,
) -> Result<(), Box<dyn Error>> {
    nibble.onchain_connectors.push(OnChainConnector {
        name: name.to_string(),
        address,
        public,
        transactions: vec![],
    });
    Ok(())
}

impl OnChainConnector {
    pub fn add_transaction(
        &mut self,
        function_signature: &str,
        params: Vec<Value>,
        gas_options: GasOptions,
    ) -> Result<(), Box<dyn Error>> {
        self.transactions.push(OnChainTransaction {
            function_signature: function_signature.to_string(),
            params,
            gas_options,
        });
        Ok(())
    }

    pub async fn execute_transactions(
        &self,
        client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    ) -> Result<(), Box<dyn Error>> {
        for tx in &self.transactions {
            let encoded_data = self.encode_function_call(&tx.function_signature, &tx.params)?;

            let mut tx_request = Eip1559TransactionRequest::new()
                .to(NameOrAddress::Address(self.address))
                .data(encoded_data);

            if let Some(gas_limit) = tx.gas_options.gas_limit {
                tx_request = tx_request.gas(gas_limit);
            }
            if let Some(max_fee) = tx.gas_options.max_fee_per_gas {
                tx_request = tx_request.max_fee_per_gas(max_fee);
            }
            if let Some(priority_fee) = tx.gas_options.max_priority_fee_per_gas {
                tx_request = tx_request.max_priority_fee_per_gas(priority_fee);
            }
            if let Some(nonce) = tx.gas_options.nonce {
                tx_request = tx_request.nonce(nonce);
            }

            let pending_tx = client.send_transaction(tx_request, None).await?;
            let receipt = pending_tx.await?;

            match receipt {
                Some(r) => println!("Transaction executed with status: {:?}", r.status),
                None => println!("Transaction was not mined"),
            }
        }
        Ok(())
    }

    fn encode_function_call(
        &self,
        function_signature: &str,
        params: &[Value],
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let abi =
            abi::AbiParser::default().parse_str(&format!("function {};", function_signature))?;
        let func = abi.functions().next().ok_or("Function not found")?;

        let tokens = params
            .iter()
            .map(|p| convert_value_to_token(p))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(func.encode_input(&tokens)?)
    }
}
