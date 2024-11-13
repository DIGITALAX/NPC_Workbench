import { newMockEvent } from "matchstick-as"
import { ethereum, Address } from "@graphprotocol/graph-ts"
import { AgentMemeDeployed } from "../generated/AgentMemeFactory/AgentMemeFactory"

export function createAgentMemeDeployedEvent(
  admin: Address,
  accessControlAddress: Address,
  fhe: Address,
  data: Address,
  token: Address
): AgentMemeDeployed {
  let agentMemeDeployedEvent = changetype<AgentMemeDeployed>(newMockEvent())

  agentMemeDeployedEvent.parameters = new Array()

  agentMemeDeployedEvent.parameters.push(
    new ethereum.EventParam("admin", ethereum.Value.fromAddress(admin))
  )
  agentMemeDeployedEvent.parameters.push(
    new ethereum.EventParam(
      "accessControlAddress",
      ethereum.Value.fromAddress(accessControlAddress)
    )
  )
  agentMemeDeployedEvent.parameters.push(
    new ethereum.EventParam("fhe", ethereum.Value.fromAddress(fhe))
  )
  agentMemeDeployedEvent.parameters.push(
    new ethereum.EventParam("data", ethereum.Value.fromAddress(data))
  )
  agentMemeDeployedEvent.parameters.push(
    new ethereum.EventParam("token", ethereum.Value.fromAddress(token))
  )

  return agentMemeDeployedEvent
}
