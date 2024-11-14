// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";

contract AgentMemeData is Initializable {
    address[] private _agentAddresses;
    bool public initialized;

    error AlreadyInitialized();

    function initialize(address[] memory agentAddresses) external {
        if (initialized) {
            revert AlreadyInitialized();
        }
        initialized = true;
        _agentAddresses = agentAddresses;
    }

    function getActiveAgentAddresses() public view returns (address[] memory) {
        return _agentAddresses;
    }
}
