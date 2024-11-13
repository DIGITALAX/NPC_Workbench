pub fn chain_to_network(chain: u64) -> &'static str {
    match chain {
        1 => "mainnet",
        137 => "polygon",
        8002 => "polygon-amoy",
        42161 => "arbitrum-one",
        43114 => "avalanche",
        _ => "unknown-network",
    }
}