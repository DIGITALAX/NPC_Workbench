// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import "./AgentMemeAccessControls.sol";
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";

contract AgentMemeFHE  is Initializable {
    AgentMemeAccessControls public accessControls;
    bool public initialized;
    error AlreadyInitialized();

    function initialize(address accessControlsAddress) external {
        if (initialized) {
            revert AlreadyInitialized();
        }
        initialized = true;
        accessControls = AgentMemeAccessControls(accessControlsAddress);
    }
}
