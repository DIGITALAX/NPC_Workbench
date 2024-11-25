import {
  assert,
  describe,
  test,
  clearStore,
  beforeAll,
  afterAll
} from "matchstick-as/assembly/index"
import { Address, Bytes, BigInt } from "@graphprotocol/graph-ts"
import { NibbleDeployed } from "../generated/schema"
import { NibbleDeployed as NibbleDeployedEvent } from "../generated/NibbleFactory/NibbleFactory"
import { handleNibbleDeployed } from "../src/nibble-factory"
import { createNibbleDeployedEvent } from "./nibble-factory-utils"

// Tests structure (matchstick-as >=0.5.0)
// https://thegraph.com/docs/en/developer/matchstick/#tests-structure-0-5-0

describe("Describe entity assertions", () => {
  beforeAll(() => {
    let storageContract = Address.fromString(
      "0x0000000000000000000000000000000000000001"
    )
    let listenersContract = Address.fromString(
      "0x0000000000000000000000000000000000000001"
    )
    let conditionsContract = Address.fromString(
      "0x0000000000000000000000000000000000000001"
    )
    let evaluationsContract = Address.fromString(
      "0x0000000000000000000000000000000000000001"
    )
    let agentsContract = Address.fromString(
      "0x0000000000000000000000000000000000000001"
    )
    let connectorsContract = Address.fromString(
      "0x0000000000000000000000000000000000000001"
    )
    let fheGatesContract = Address.fromString(
      "0x0000000000000000000000000000000000000001"
    )
    let accessControlsContract = Address.fromString(
      "0x0000000000000000000000000000000000000001"
    )
    let workflowsContract = Address.fromString(
      "0x0000000000000000000000000000000000000001"
    )
    let id = Bytes.fromI32(1234567890)
    let count = BigInt.fromI32(234)
    let newNibbleDeployedEvent = createNibbleDeployedEvent(
      storageContract,
      listenersContract,
      conditionsContract,
      evaluationsContract,
      agentsContract,
      connectorsContract,
      fheGatesContract,
      accessControlsContract,
      workflowsContract,
      id,
      count
    )
    handleNibbleDeployed(newNibbleDeployedEvent)
  })

  afterAll(() => {
    clearStore()
  })

  // For more test scenarios, see:
  // https://thegraph.com/docs/en/developer/matchstick/#write-a-unit-test

  test("NibbleDeployed created and stored", () => {
    assert.entityCount("NibbleDeployed", 1)

    // 0xa16081f360e3847006db660bae1c6d1b2e17ec2a is the default address used in newMockEvent() function
    assert.fieldEquals(
      "NibbleDeployed",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "storageContract",
      "0x0000000000000000000000000000000000000001"
    )
    assert.fieldEquals(
      "NibbleDeployed",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "listenersContract",
      "0x0000000000000000000000000000000000000001"
    )
    assert.fieldEquals(
      "NibbleDeployed",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "conditionsContract",
      "0x0000000000000000000000000000000000000001"
    )
    assert.fieldEquals(
      "NibbleDeployed",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "evaluationsContract",
      "0x0000000000000000000000000000000000000001"
    )
    assert.fieldEquals(
      "NibbleDeployed",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "agentsContract",
      "0x0000000000000000000000000000000000000001"
    )
    assert.fieldEquals(
      "NibbleDeployed",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "connectorsContract",
      "0x0000000000000000000000000000000000000001"
    )
    assert.fieldEquals(
      "NibbleDeployed",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "fheGatesContract",
      "0x0000000000000000000000000000000000000001"
    )
    assert.fieldEquals(
      "NibbleDeployed",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "accessControlsContract",
      "0x0000000000000000000000000000000000000001"
    )
    assert.fieldEquals(
      "NibbleDeployed",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "workflowsContract",
      "0x0000000000000000000000000000000000000001"
    )
    assert.fieldEquals(
      "NibbleDeployed",
      "0xa16081f360e3847006db660bae1c6d1b2e17ec2a-1",
      "count",
      "234"
    )

    // More assert options:
    // https://thegraph.com/docs/en/developer/matchstick/#asserts
  })
})
