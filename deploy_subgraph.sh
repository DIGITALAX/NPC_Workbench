#!/bin/bash

SUBGRAPH_NAME=$1
ACCESS_CONTROL_ADDRESS=$2
FHE_ADDRESS=$3
DATA_ADDRESS=$4
TOKEN_ADDRESS=$5
NETWORK=$6

OUTPUT_DIR="./subgraphs/$SUBGRAPH_NAME"

graph init --from-contract $ACCESS_CONTROL_ADDRESS --network $NETWORK $SUBGRAPH_NAME --output-dir $OUTPUT_DIR

cat <<EOT >> $OUTPUT_DIR/subgraph.yaml

- kind: ethereum/contract
  name: FHE
  network: $NETWORK
  source:
    address: "$FHE_ADDRESS"
    abi: FHE
- kind: ethereum/contract
  name: Data
  network: $NETWORK
  source:
    address: "$DATA_ADDRESS"
    abi: Data
- kind: ethereum/contract
  name: Token
  network: $NETWORK
  source:
    address: "$TOKEN_ADDRESS"
    abi: Token
EOT

cd $OUTPUT_DIR
graph codegen && graph build && graph deploy --studio $SUBGRAPH_NAME
