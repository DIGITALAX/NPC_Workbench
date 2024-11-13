import { AgentMemeDeployed as AgentMemeDeployedEvent } from "../generated/AgentMemeFactory/AgentMemeFactory"
import { AgentMemeDeployed } from "../generated/schema"

export function handleAgentMemeDeployed(event: AgentMemeDeployedEvent): void {
  let entity = new AgentMemeDeployed(
    event.transaction.hash.concatI32(event.logIndex.toI32())
  )
  entity.admin = event.params.admin
  entity.accessControlAddress = event.params.accessControlAddress
  entity.fhe = event.params.fhe
  entity.data = event.params.data
  entity.token = event.params.token

  entity.blockNumber = event.block.number
  entity.blockTimestamp = event.block.timestamp
  entity.transactionHash = event.transaction.hash

  entity.save()
}
