// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

contract AgentMemeLibrary {
    struct Agent {
        bytes id;
    }

    struct Nibble {
        address[4] contracts;
        bytes id;
        uint256 count;
    }
}
