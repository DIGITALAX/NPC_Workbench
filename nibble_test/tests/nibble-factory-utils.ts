import { newMockEvent } from "matchstick-as"
import { ethereum, Address, Bytes, BigInt } from "@graphprotocol/graph-ts"
import { NibbleDeployed } from "../generated/NibbleFactory/NibbleFactory"

export function createNibbleDeployedEvent(
  storageContract: Address,
  listenersContract: Address,
  conditionsContract: Address,
  evaluationsContract: Address,
  agentsContract: Address,
  connectorsContract: Address,
  fheGatesContract: Address,
  accessControlsContract: Address,
  workflowsContract: Address,
  id: Bytes,
  count: BigInt
): NibbleDeployed {
  let nibbleDeployedEvent = changetype<NibbleDeployed>(newMockEvent())

  nibbleDeployedEvent.parameters = new Array()

  nibbleDeployedEvent.parameters.push(
    new ethereum.EventParam(
      "storageContract",
      ethereum.Value.fromAddress(storageContract)
    )
  )
  nibbleDeployedEvent.parameters.push(
    new ethereum.EventParam(
      "listenersContract",
      ethereum.Value.fromAddress(listenersContract)
    )
  )
  nibbleDeployedEvent.parameters.push(
    new ethereum.EventParam(
      "conditionsContract",
      ethereum.Value.fromAddress(conditionsContract)
    )
  )
  nibbleDeployedEvent.parameters.push(
    new ethereum.EventParam(
      "evaluationsContract",
      ethereum.Value.fromAddress(evaluationsContract)
    )
  )
  nibbleDeployedEvent.parameters.push(
    new ethereum.EventParam(
      "agentsContract",
      ethereum.Value.fromAddress(agentsContract)
    )
  )
  nibbleDeployedEvent.parameters.push(
    new ethereum.EventParam(
      "connectorsContract",
      ethereum.Value.fromAddress(connectorsContract)
    )
  )
  nibbleDeployedEvent.parameters.push(
    new ethereum.EventParam(
      "fheGatesContract",
      ethereum.Value.fromAddress(fheGatesContract)
    )
  )
  nibbleDeployedEvent.parameters.push(
    new ethereum.EventParam(
      "accessControlsContract",
      ethereum.Value.fromAddress(accessControlsContract)
    )
  )
  nibbleDeployedEvent.parameters.push(
    new ethereum.EventParam(
      "workflowsContract",
      ethereum.Value.fromAddress(workflowsContract)
    )
  )
  nibbleDeployedEvent.parameters.push(
    new ethereum.EventParam("id", ethereum.Value.fromBytes(id))
  )
  nibbleDeployedEvent.parameters.push(
    new ethereum.EventParam("count", ethereum.Value.fromUnsignedBigInt(count))
  )

  return nibbleDeployedEvent
}
