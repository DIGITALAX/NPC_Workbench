// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

contract NibbleLibrary {
    error InvalidInitializer();
    error OnlyAdmin();
    error AdminCantBeZero();
    error InvalidRole();
    error InvalidLengths();

    struct Nibble {
        address storageContract;
        address listenerContract;
        address conditionContract;
        address evaluationContract;
        address agentContract;
        address connectorContract;
        address fheGateContract;
        address accessControlContract;
        bytes id;
        uint256 count;
    }

    struct Condition {
        bytes id;
        string metadata;
        bool encrypted;
    }

    struct Evaluation {
        bytes id;
        string metadata;
        bool encrypted;
    }

    struct FHEGate {
        bytes id;
        string metadata;
        bool encrypted;
    }

    struct Listener {
        bytes id;
        string metadata;
        bool encrypted;
    }

    struct Connector {
        bytes id;
        string metadata;
        bool encrypted;
        bool onChain;
    }

    struct Agent {
        bytes id;
        string metadata;
        address wallet;
        bool encrypted;
        bool writer;
    }

    struct ModifyAdapters {
        Condition[] conditions;
        Listener[] listeners;
        Connector[] connectors;
        Agent[] agents;
        Evaluation[] evaluations;
        FHEGate[] fheGates;
    }

    struct RemoveAdapters {
        bytes[] conditions;
        bytes[] listeners;
        bytes[] connectors;
        bytes[] agents;
        bytes[] evaluations;
        bytes[] fheGates;
    }
}
