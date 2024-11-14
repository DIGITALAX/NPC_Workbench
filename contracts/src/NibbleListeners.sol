// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import "./NibbleLibrary.sol";

contract NibbleListeners is Initializable {
    address public nibbleFactory;
    bool public initialized;

    modifier OnlyNibbleFactory() {
        if (msg.sender != nibbleFactory) {
            revert NibbleLibrary.InvalidInitializer();
        }
        _;
    }

    constructor(address nibbleFactoryAddress) {
        nibbleFactory = nibbleFactoryAddress;
    }

    function initialize() external OnlyNibbleFactory {
        if (initialized) {
            revert NibbleLibrary.AlreadyInitialized();
        }
        initialized = true;
    }
}
