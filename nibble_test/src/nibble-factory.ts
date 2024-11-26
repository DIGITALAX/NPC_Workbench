import { Address, Bytes } from "@graphprotocol/graph-ts";
import { NibbleDeployed as NibbleDeployedEvent } from "../generated/NibbleFactory/NibbleFactory";
import { ContractInfo, NibbleDeployed } from "../generated/schema";

export function handleNibbleDeployed(event: NibbleDeployedEvent): void {
  let entity = new NibbleDeployed(event.params.id);

  let contracts: Bytes[] = [
    event.params.storageContract,
    event.params.listenersContract,
    event.params.conditionsContract,
    event.params.evaluationsContract,
    event.params.agentsContract,
    event.params.connectorsContract,
    event.params.fheGatesContract,
    event.params.accessControlsContract,
    event.params.workflowsContract,
  ];

  let names: string[] = [
    "NibbleStorage",
    "NibbleListeners",
    "NibbleConditions",
    "NibbleEvaluations",
    "NibbleAgents",
    "NibbleConnectors",
    "NibbleFHEGates",
    "NibbleAccessControls",
    "NibbleWorkflows",
  ];

  let contractInfoIds: Bytes[] = [];

  for (let i = 0; i < contracts.length; i++) {
    let info = new ContractInfo(contracts[i]);

    info.name = names[i];
    info.address = contracts[i];

    info.save();

    contractInfoIds.push(info.id);
  }

  entity.contracts = contractInfoIds;
  entity.count = event.params.count;

  entity.blockNumber = event.block.number;
  entity.blockTimestamp = event.block.timestamp;
  entity.transactionHash = event.transaction.hash;

  entity.save();
}
