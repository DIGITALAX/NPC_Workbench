import { NibbleDeployed as NibbleDeployedEvent } from "../generated/NibbleFactory/NibbleFactory"
import { NibbleDeployed } from "../generated/schema"

export function handleNibbleDeployed(event: NibbleDeployedEvent): void {
  let entity = new NibbleDeployed(
    event.transaction.hash.concatI32(event.logIndex.toI32())
  )
  entity.storageContract = event.params.storageContract
  entity.listenersContract = event.params.listenersContract
  entity.conditionsContract = event.params.conditionsContract
  entity.evaluationsContract = event.params.evaluationsContract
  entity.agentsContract = event.params.agentsContract
  entity.connectorsContract = event.params.connectorsContract
  entity.fheGatesContract = event.params.fheGatesContract
  entity.accessControlsContract = event.params.accessControlsContract
  entity.workflowsContract = event.params.workflowsContract
  entity.NibbleFactory_id = event.params.id
  entity.count = event.params.count

  entity.blockNumber = event.block.number
  entity.blockTimestamp = event.block.timestamp
  entity.transactionHash = event.transaction.hash

  entity.save()
}
