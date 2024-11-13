import {
  assert,
  describe,
  test,
  clearStore,
  beforeAll,
  afterAll
} from "matchstick-as/assembly/index"
import { Address } from "@graphprotocol/graph-ts"
import { AgentMemeDeployed } from "../generated/schema"
import { AgentMemeDeployed as AgentMemeDeployedEvent } from "../generated/AgentMemeFactory/AgentMemeFactory"
import { handleAgentMemeDeployed } from "../src/agent-meme-factory"
import { createAgentMemeDeployedEvent } from "./agent-meme-factory-utils"

// Tests structure (matchstick-as >=0.5.0)
// https://thegraph.com/docs/en/developer/matchstick/#tests-structure-0-5-0

describe("Describe entity assertions", () => {
  beforeAll(() => {
    let admin = Address.fromString("0x0000000000000000000000000000000000000001")
    let accessControlAddress = Address.fromString(
      "0x0000000000000000000000000000000000000001"
    )
    let fhe = Address.fromString("0x0000000000000000000000000000000000000001")
    let data = Address.fromString("0x0000000000000000000000000000000000000001")
    let token = Address.fromString("0x0000000000000000000000000000000000000001")
    let newAgentMemeDeployedEvent = createAgentMemeDeployedEvent(
      admin,
      accessControlAddress,
      fhe,
      data,
      token
    )
    handleAgentMemeDeployed(newAgentMemeDeployedEvent)
  })

  afterAll(() => {
    clearStore()
  })

  // For more test scenarios, see:
  // https://thegraph.com/docs/en/developer/matchstick/#write-a-unit-test

  test("AgentMemeDeployed created and stored", () => {
    assert.entityCount("AgentMemeDeployed", 1)

    // 0xa16081f360e3847006db660bae1c6d1b2e17ec2a is the default address used in newMockEvent() function
    assert.fieldEquals(
      "AgentMemeDeployed",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "admin",
      "0x0000000000000000000000000000000000000001"
    )
    assert.fieldEquals(
      "AgentMemeDeployed",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "accessControlAddress",
      "0x0000000000000000000000000000000000000001"
    )
    assert.fieldEquals(
      "AgentMemeDeployed",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "fhe",
      "0x0000000000000000000000000000000000000001"
    )
    assert.fieldEquals(
      "AgentMemeDeployed",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "data",
      "0x0000000000000000000000000000000000000001"
    )
    assert.fieldEquals(
      "AgentMemeDeployed",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "token",
      "0x0000000000000000000000000000000000000001"
    )

    // More assert options:
    // https://thegraph.com/docs/en/developer/matchstick/#asserts
  })
})
