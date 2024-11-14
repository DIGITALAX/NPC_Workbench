// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

contract NibbleLibrary {
    error AlreadyInitialized();
    error InvalidInitializer();
    error OnlyAdmin();
    error AdminCantBeZero();

    struct Agent {
        bytes id;
    }

    struct Nibble {
        address storageContract;
        address listenerContract;
        address conditionContract;
        address evaluationContract;
        address agentContract;
        address adapterContract;
        address accessControlContract;
        bytes id;
        uint256 count;
    }
}
