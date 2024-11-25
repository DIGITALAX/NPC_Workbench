// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../src/NibbleFactory.sol";
import "../src/NibbleLibrary.sol";
import "@openzeppelin/contracts/proxy/Clones.sol";
import "../src/NibbleAccessControls.sol";
import "../src/NibbleStorage.sol";
import "../src/NibbleConditions.sol";
import "../src/NibbleListeners.sol";
import "../src/NibbleEvaluations.sol";
import "../src/NibbleAgents.sol";
import "../src/NibbleConnectors.sol";
import "../src/NibbleFHEGates.sol";
import "../src/NibbleWorkflows.sol";

contract NibbleFactoryTest is Test {
    NibbleFactory factory;

    address storageImplementation = address(new NibbleStorage());
    address listenersImplementation = address(new NibbleListeners());
    address conditionsImplementation = address(new NibbleConditions());
    address agentsImplementation = address(new NibbleAgents());
    address evaluationsImplementation = address(new NibbleEvaluations());
    address connectorsImplementation = address(new NibbleConnectors());
    address accessControlsImplementation = address(new NibbleAccessControls());
    address fheGatesImplementation = address(new NibbleFHEGates());
    address workflowsImplementation = address(new NibbleWorkflows());

    function setUp() public {
        factory = new NibbleFactory(
            storageImplementation,
            listenersImplementation,
            conditionsImplementation,
            agentsImplementation,
            evaluationsImplementation,
            connectorsImplementation,
            accessControlsImplementation,
            fheGatesImplementation,
            workflowsImplementation
        );
    }

    function testDeployFromFactory() public {
        vm.prank(address(0xDEADBEEF));
        (
            address[9] memory deployedContracts,
            bytes memory id,
            uint256 count
        ) = factory.deployFromFactory();


        assertEq(deployedContracts.length, 9);
        assertGt(id.length, 0);
        assertEq(count, 1);
    }
}
