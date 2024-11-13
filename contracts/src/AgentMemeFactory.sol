// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "./AgentMemeAccessControls.sol";
import "./AgentMemeFHE.sol";
import "./AgentMemeERC20.sol";
import "./AgentMemeData.sol";

contract AgentMemeFactory {
    event AgentMemeDeployed(
        address indexed admin,
        address accessControlAddress,
        address fhe,
        address data,
        address token
    );

    function deployFromFactory(
        address[] memory agentWriters,
        address[] memory agentReaders,
        address[] memory agentAdmins,
        address[] memory agentTokenDistributors,
        string memory tokenName,
        string memory tokenSymbol,
        uint256 tokenSupply
    ) external returns (address[4] memory) {
        AgentMemeAccessControls accessControl = new AgentMemeAccessControls(
            msg.sender,
            agentWriters,
            agentReaders,
            agentAdmins,
            agentTokenDistributors
        );
        AgentMemeFHE fhe = new AgentMemeFHE();

        address[] memory agentAddresses = _combineAgentAddresses(
            agentWriters,
            agentReaders,
            agentAdmins,
            agentTokenDistributors
        );

        AgentMemeData data = new AgentMemeData(agentAddresses);
        AgentMemeERC20 token = new AgentMemeERC20(
            tokenName,
            tokenSymbol,
            address(accessControl),
            tokenSupply
        );

        emit AgentMemeDeployed(
            msg.sender,
            address(accessControl),
            address(fhe),
            address(data),
            address(token)
        );

        return [
            address(accessControl),
            address(fhe),
            address(data),
            address(token)
        ];
    }

    function _combineAgentAddresses(
        address[] memory agentWriters,
        address[] memory agentReaders,
        address[] memory agentAdmins,
        address[] memory agentTokenDistributors
    ) internal pure returns (address[] memory) {
        uint256 totalLength = agentWriters.length +
            agentReaders.length +
            agentAdmins.length +
            agentTokenDistributors.length;
        address[] memory agentAddresses = new address[](totalLength);

        uint256 index = 0;
        for (uint256 i = 0; i < agentWriters.length; i++) {
            agentAddresses[index++] = agentWriters[i];
        }
        for (uint256 j = 0; j < agentReaders.length; j++) {
            agentAddresses[index++] = agentReaders[j];
        }
        for (uint256 k = 0; k < agentAdmins.length; k++) {
            agentAddresses[index++] = agentAdmins[k];
        }
        for (uint256 l = 0; l < agentTokenDistributors.length; l++) {
            agentAddresses[index++] = agentTokenDistributors[l];
        }

        return agentAddresses;
    }
}
