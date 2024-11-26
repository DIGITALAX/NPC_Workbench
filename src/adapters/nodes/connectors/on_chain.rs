use crate::{nibble::Adaptable, utils::generate_unique_id};
use ethers::{
    abi,
    prelude::*,
    types::{Address, Eip1559TransactionRequest, NameOrAddress, U256},
    utils::hex,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{error::Error, io, sync::Arc};
use transaction::eip2718::TypedTransaction;

#[derive(Debug, Clone)]
pub struct OnChainConnector {
    pub name: String,
    pub id: String,
    pub address: Option<Address>,
    pub encrypted: bool,
    pub abi: Option<abi::Abi>,
    pub bytecode: Option<Bytes>,
    pub chain: Chain,
    pub gas_options: Option<GasOptions>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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
    name: &str,
    address: Option<Address>,
    encrypted: bool,
    owner_address: &H160,
    bytecode: Option<Bytes>,
    abi: Option<abi::Abi>,
    chain: Chain,
    gas_options: Option<GasOptions>,
) -> Result<OnChainConnector, Box<dyn Error + Send + Sync>> {
    let on_chain = OnChainConnector {
        name: name.to_string(),
        id: generate_unique_id(owner_address),
        address,
        encrypted,
        bytecode,
        abi,
        chain,
        gas_options,
    };
    Ok(on_chain)
}

impl OnChainConnector {
    pub fn to_json(&self) -> Map<String, Value> {
        let mut map = Map::new();

        map.insert("name".to_string(), Value::String(self.name.clone()));
        map.insert("id".to_string(), Value::String(hex::encode(&self.id)));
        map.insert(
            "address".to_string(),
            Value::String(
                self.address
                    .map(|addr| format!("{:?}", addr))
                    .unwrap_or_else(|| "None".to_string()),
            ),
        );
        map.insert("encrypted".to_string(), Value::Bool(self.encrypted));
        map.insert(
            "chain".to_string(),
            Value::String(format!("{:?}", self.chain)),
        );

        if let Some(abi) = &self.abi {
            map.insert("abi".to_string(), Value::String(format!("{:?}", abi)));
        }

        if let Some(bytecode) = &self.bytecode {
            map.insert("bytecode".to_string(), Value::String(hex::encode(bytecode)));
        }

        if let Some(gas_options) = &self.gas_options {
            let mut gas_map = Map::new();
            if let Some(max_fee) = gas_options.max_fee_per_gas {
                gas_map.insert(
                    "max_fee_per_gas".to_string(),
                    Value::String(max_fee.to_string()),
                );
            }
            if let Some(max_priority_fee) = gas_options.max_priority_fee_per_gas {
                gas_map.insert(
                    "max_priority_fee_per_gas".to_string(),
                    Value::String(max_priority_fee.to_string()),
                );
            }
            if let Some(gas_limit) = gas_options.gas_limit {
                gas_map.insert(
                    "gas_limit".to_string(),
                    Value::String(gas_limit.to_string()),
                );
            }
            if let Some(nonce) = gas_options.nonce {
                gas_map.insert("nonce".to_string(), Value::String(nonce.to_string()));
            }
            map.insert("gas_options".to_string(), Value::Object(gas_map));
        }

        map
    }

    pub async fn execute_onchain_connector(
        &self,
        provider: Provider<Http>,
        wallet: LocalWallet,
        method_name: Option<&str>,
        params: Option<Vec<Value>>,
    ) -> Result<Option<Value>, Box<dyn Error + Send + Sync>> {
        let client = SignerMiddleware::new(provider.clone(), wallet.clone());
        let client = Arc::new(client);

        if let Some(method) = method_name {
            if let (Some(address), Some(abi)) = (&self.address, &self.abi) {
                let contract = Contract::new(*address, abi.clone(), client.clone());

                let decoded_params: Vec<abi::Token> = params
                    .unwrap_or_default()
                    .into_iter()
                    .map(|param| {
                        serde_json::from_value::<abi::Token>(param).map_err(|e| e.to_string())
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                let method_call = contract.method::<_, Vec<abi::Token>>(method, decoded_params)?;
                let tx_request = method_call.tx;

                let tx_request = if let Some(gas) = &self.gas_options {
                    Eip1559TransactionRequest {
                        from: Some(client.address()),
                        to: Some(NameOrAddress::Address(*address)),
                        gas: gas.gas_limit.or(tx_request.gas().copied()),
                        value: tx_request.value().copied(),
                        data: tx_request.data().cloned(),
                        max_priority_fee_per_gas: gas
                            .max_priority_fee_per_gas
                            .or_else(|| Some(2_000_000_000u64.into())),
                        max_fee_per_gas: gas
                            .max_fee_per_gas
                            .or_else(|| Some(100_000_000_000u64.into())),
                        nonce: gas.nonce.or_else(|| None),
                        chain_id: Some(self.chain.into()),
                        ..Default::default()
                    }
                } else {
                    Eip1559TransactionRequest {
                        from: Some(client.address()),
                        to: Some(NameOrAddress::Address(*address)),
                        gas: tx_request.gas().copied(),
                        value: tx_request.value().copied(),
                        data: tx_request.data().cloned(),
                        max_priority_fee_per_gas: Some(2_000_000_000u64.into()),
                        max_fee_per_gas: Some(100_000_000_000u64.into()),
                        nonce: None,
                        chain_id: Some(self.chain.into()),
                        ..Default::default()
                    }
                };

                let pending_tx = client
                    .send_transaction(tx_request, None)
                    .await
                    .map_err(|e| {
                        eprintln!("Error sending the transaction: {:?}", e);
                        Box::<dyn Error + Send + Sync>::from(format!(
                            "Error sending the transaction: {}",
                            e
                        ))
                    })?;

                let receipt = pending_tx.await?;
                if let Some(receipt) = receipt {
                    if receipt.status == Some(U64::from(1)) {
                        println!("Transaction succeeded: {:?}", receipt.transaction_hash);
                        Ok(Some(Value::String(format!(
                            "Transaction Hash: {:?}",
                            receipt.transaction_hash
                        ))))
                    } else {
                        eprintln!("Transaction failed: {:?}", receipt);
                        Err("Transaction execution failed".into())
                    }
                } else {
                    Err("Transaction was not mined".into())
                }
            } else {
                Err("Contract address or ABI is missing".into())
            }
        } else {
            if let (Some(abi), Some(bytecode)) = (&self.abi, &self.bytecode) {
                let factory = ContractFactory::new(abi.clone(), bytecode.clone(), client.clone());

                let constructor_args: Vec<abi::Token> = match params {
                    Some(param_list) => param_list
                        .into_iter()
                        .map(|param| {
                            serde_json::from_value::<abi::Token>(param)
                                .map_err(|e| format!("Error decoding arguments: {}", e))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    None => vec![],
                };

                let deployer = factory.deploy(constructor_args)?;

                let mut tx = deployer.tx.clone();
                if let TypedTransaction::Eip1559(ref mut request) = tx {
                    if let Some(ref gas_options) = self.gas_options {
                        request.max_fee_per_gas = gas_options
                            .max_fee_per_gas
                            .or_else(|| Some(100_000_000_000u64.into()));
                        request.max_priority_fee_per_gas = gas_options
                            .max_priority_fee_per_gas
                            .or_else(|| Some(2_000_000_000u64.into()));
                        request.gas = gas_options.gas_limit.or_else(|| Some(2_000_000u64.into()));
                        request.nonce = gas_options.nonce;
                    } else {
                        request.max_fee_per_gas = Some(100_000_000_000u64.into());
                        request.max_priority_fee_per_gas = Some(2_000_000_000u64.into());
                        request.gas = Some(2_000_000u64.into());
                        request.nonce = None;
                    }
                } else {
                    panic!("The transaction is not of type EIP-1559");
                }

                let pending_tx = client.send_transaction(tx, None).await?;

                match pending_tx.await {
                    Ok(contract) => match contract {
                        Some(tx) => {
                            println!("Contract deployed at: {:?}", tx.contract_address);
                            Ok(Some(Value::String(
                                tx.contract_address.unwrap().to_string(),
                            )))
                        }
                        None => {
                            eprintln!("Error getting contract address");
                            Err(Box::new(io::Error::new(
                                io::ErrorKind::Other,
                                "Error getting contract address",
                            )))
                        }
                    },
                    Err(e) => {
                        eprintln!("Error deploying contract: {:?}", e);
                        Err(Box::new(e))
                    }
                }
            } else {
                Err("ABI or Bytecode is missing for contract deployment".into())
            }
        }
    }
}

impl Adaptable for OnChainConnector {
    fn name(&self) -> &str {
        &self.name
    }
    fn id(&self) -> &str {
        &self.id
    }
}
