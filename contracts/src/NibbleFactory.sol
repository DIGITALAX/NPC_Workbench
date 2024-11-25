// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/proxy/Clones.sol";
import "./NibbleLibrary.sol";
import "./NibbleConnectors.sol";
import "./NibbleConditions.sol";
import "./NibbleListeners.sol";
import "./NibbleEvaluations.sol";
import "./NibbleAgents.sol";
import "./NibbleAccessControls.sol";
import "./NibbleStorage.sol";
import "./NibbleFHEGates.sol";

contract NibbleFactory {
    uint256 public count;
    address public listenersImplementation;
    address public conditionsImplementation;
    address public agentsImplementation;
    address public storageImplementation;
    address public evaluationsImplementation;
    address public connectorsImplementation;
    address public fheGatesImplementation;
    address public accessControlsImplementation;
    address public workflowsImplementation;

    mapping(address => NibbleLibrary.Nibble[]) private _nibbles;
    mapping(address => uint256) private _nibbleCount;

    event NibbleDeployed(
        address storageContract,
        address listenersContract,
        address conditionsContract,
        address evaluationsContract,
        address agentsContract,
        address connectorsContract,
        address fheGatesContract,
        address accessControlsContract,
        address workflowsContract,
        bytes id,
        uint256 count
    );

    constructor(
        address storageImp,
        address listenersImp,
        address conditionsImp,
        address agentsImp,
        address evaluationsImp,
        address connectorsImp,
        address accessControlsImp,
        address fheGatesImp,
        address workflowsImp
    ) {
        listenersImplementation = listenersImp;
        conditionsImplementation = conditionsImp;
        agentsImplementation = agentsImp;
        storageImplementation = storageImp;
        evaluationsImplementation = evaluationsImp;
        connectorsImplementation = connectorsImp;
        accessControlsImplementation = accessControlsImp;
        fheGatesImplementation = fheGatesImp;
        workflowsImplementation = workflowsImp;
        count = 0;
    }

    function deployFromFactory()
        external
        returns (address[9] memory, bytes memory, uint256)
    {
        address _newStorage = Clones.clone(storageImplementation);
        address _newConditions = Clones.clone(conditionsImplementation);
        address _newListeners = Clones.clone(listenersImplementation);
        address _newConnectors = Clones.clone(connectorsImplementation);
        address _newAgents = Clones.clone(agentsImplementation);
        address _newEvaluations = Clones.clone(evaluationsImplementation);
        address _newFHEGates = Clones.clone(fheGatesImplementation);
        address _newAccessControls = Clones.clone(accessControlsImplementation);
        address _newWorkflows = Clones.clone(workflowsImplementation);

        (bool success, ) = _newAccessControls.call(
            abi.encodeWithSignature(
                "initialize(address,address)",
                address(this),
                msg.sender
            )
        );
        if (!success) {
            revert NibbleLibrary.AccessControlInitializationFailed();
        }

        (success, ) = _newStorage.call(
            abi.encodeWithSignature(
                "initialize(address,address,address,address,address,address,address,address,address)",
                address(this),
                _newAccessControls,
                _newConditions,
                _newListeners,
                _newConnectors,
                _newAgents,
                _newEvaluations,
                _newFHEGates,
                _newWorkflows
            )
        );
        if (!success) {
            revert NibbleLibrary.StorageInitializationFailed();
        }

        (success, ) = _newConditions.call(
            abi.encodeWithSignature(
                "initialize(address,address,address)",
                address(this),
                _newAccessControls,
                _newStorage
            )
        );
        if (!success) {
            revert NibbleLibrary.ConditionsInitializationFailed();
        }

        (success, ) = _newListeners.call(
            abi.encodeWithSignature(
                "initialize(address,address,address)",
                address(this),
                _newAccessControls,
                _newStorage
            )
        );
        if (!success) {
            revert NibbleLibrary.ListenersInitializationFailed();
        }

        (success, ) = _newEvaluations.call(
            abi.encodeWithSignature(
                "initialize(address,address,address)",
                address(this),
                _newAccessControls,
                _newStorage
            )
        );
        if (!success) {
            revert NibbleLibrary.EvaluationsInitializationFailed();
        }

        (success, ) = _newConnectors.call(
            abi.encodeWithSignature(
                "initialize(address,address,address)",
                address(this),
                _newAccessControls,
                _newStorage
            )
        );
        if (!success) {
            revert NibbleLibrary.ConnectorsInitializationFailed();
        }

        (success, ) = _newFHEGates.call(
            abi.encodeWithSignature(
                "initialize(address,address,address)",
                address(this),
                _newAccessControls,
                _newStorage
            )
        );
        if (!success) {
            revert NibbleLibrary.FHEGatesInitializationFailed();
        }

        (success, ) = _newAgents.call(
            abi.encodeWithSignature(
                "initialize(address,address,address)",
                address(this),
                _newAccessControls,
                _newStorage
            )
        );
        if (!success) {
            revert NibbleLibrary.AgentsInitializationFailed();
        }

        count++;
        bytes memory _id = _generateRandomId(
            msg.sender,
            block.timestamp,
            count
        );

        _nibbleCount[msg.sender] += 1;

        _nibbles[msg.sender].push(
            NibbleLibrary.Nibble({
                storageContract: _newStorage,
                listenerContract: _newListeners,
                conditionContract: _newConditions,
                evaluationContract: _newEvaluations,
                agentContract: _newAgents,
                connectorContract: _newConnectors,
                accessControlContract: _newAccessControls,
                fheGateContract: _newFHEGates,
                id: _id,
                count: count
            })
        );

        emit NibbleDeployed(
            _newStorage,
            _newListeners,
            _newConditions,
            _newEvaluations,
            _newAgents,
            _newConnectors,
            _newFHEGates,
            _newAccessControls,
            _newWorkflows,
            _id,
            count
        );

        return (
            [
                _newStorage,
                _newListeners,
                _newConditions,
                _newEvaluations,
                _newAgents,
                _newConnectors,
                _newFHEGates,
                _newAccessControls,
                _newWorkflows
            ],
            _id,
            count
        );
    }

    function _generateRandomId(
        address sender,
        uint256 timestamp,
        uint256 _count
    ) internal pure returns (bytes memory) {
        return
            abi.encodePacked(
                keccak256(abi.encodePacked(sender, timestamp, _count))
            );
    }

    function getDeployerNibbleCount(
        address deployer
    ) public view returns (uint256) {
        return _nibbleCount[deployer];
    }

    function getNibbleStorageContract(
        address deployer,
        uint256 index
    ) public view returns (address) {
        return _nibbles[deployer][index].storageContract;
    }

    function getNibbleAccessControlsContract(
        address deployer,
        uint256 index
    ) public view returns (address) {
        return _nibbles[deployer][index].accessControlContract;
    }

    function getNibbleAgentContract(
        address deployer,
        uint256 index
    ) public view returns (address) {
        return _nibbles[deployer][index].agentContract;
    }

    function getNibbleListenerContract(
        address deployer,
        uint256 index
    ) public view returns (address) {
        return _nibbles[deployer][index].listenerContract;
    }

    function getNibbleConditionContract(
        address deployer,
        uint256 index
    ) public view returns (address) {
        return _nibbles[deployer][index].conditionContract;
    }

    function getNibbleConnectorContract(
        address deployer,
        uint256 index
    ) public view returns (address) {
        return _nibbles[deployer][index].connectorContract;
    }

    function getNibbleEvaluationContract(
        address deployer,
        uint256 index
    ) public view returns (address) {
        return _nibbles[deployer][index].evaluationContract;
    }

    function getNibbleCount(
        address deployer,
        uint256 index
    ) public view returns (uint256) {
        return _nibbles[deployer][index].count;
    }

    function getNibbleId(
        address deployer,
        uint256 index
    ) public view returns (bytes memory) {
        return _nibbles[deployer][index].id;
    }
}
