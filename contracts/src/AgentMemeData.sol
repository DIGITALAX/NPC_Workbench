// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

contract AgentMemeData {
    address[] private _agentAddresses;

    constructor(address[] memory agentAddresses) {
        _agentAddresses = agentAddresses;
    }

    function getActiveAgentAddresses() public view returns (address[] memory) {
        return _agentAddresses;
    }
}
