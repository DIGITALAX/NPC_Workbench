// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../src/AgentMemeFactory.sol";
import "../src/AgentMemeERC20.sol";
import "../src/AgentMemeAccessControls.sol";
import "../src/AgentMemeFHE.sol";
import "../src/AgentMemeData.sol";
import "forge-std/console.sol";

contract AgentMemeFactoryTest is Test {
    AgentMemeFactory factory;

    function setUp() public {
        AgentMemeAccessControls access = new AgentMemeAccessControls();
        AgentMemeFHE fhe = new AgentMemeFHE();
        AgentMemeData data = new AgentMemeData();
        AgentMemeERC20 token = new AgentMemeERC20();

        factory = new AgentMemeFactory(
            address(access),
            address(fhe),
            address(data),
            address(token)
        );
    }

    function testDeployFromFactory() public {
        address[] memory agentWriters = new address[](1);
        agentWriters[0] = address(0x123);

        address[] memory agentReaders = new address[](1);
        agentReaders[0] = address(0x456);

        address[] memory agentAdmins = new address[](1);
        agentAdmins[0] = address(0x789);

        address[] memory agentTokenDistributors = new address[](1);
        agentTokenDistributors[0] = address(0xABC);

        string memory tokenName = "AgentToken";
        string memory tokenSymbol = "AGT";
        uint256 tokenSupply = 1000;

        address[4] memory deployedAddresses = factory.deployFromFactory(
            agentWriters,
            agentReaders,
            agentAdmins,
            agentTokenDistributors,
            tokenName,
            tokenSymbol,
            tokenSupply
        );

        address accessControlAddress = deployedAddresses[0];
        address fheAddress = deployedAddresses[1];
        address dataAddress = deployedAddresses[2];
        address tokenAddress = deployedAddresses[3];

        assertTrue(
            accessControlAddress != address(0),
            "Access control address should not be zero"
        );
        assertTrue(fheAddress != address(0), "FHE address should not be zero");
        assertTrue(
            dataAddress != address(0),
            "Data address should not be zero"
        );
        assertTrue(
            tokenAddress != address(0),
            "Token address should not be zero"
        );
        assertEq(AgentMemeERC20(tokenAddress).tokenSupply(), 1000);
        assertEq(AgentMemeERC20(tokenAddress).name(), "AgentToken");
        assertEq(AgentMemeERC20(tokenAddress).symbol(), "AGT");
    }
}
