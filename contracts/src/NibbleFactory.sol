// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/proxy/Clones.sol";
import "./NibbleLibrary.sol";
import "./NibbleAdapters.sol";
import "./NibbleConditions.sol";
import "./NibbleListeners.sol";
import "./NibbleEvaluations.sol";
import "./NibbleAgents.sol";
import "./NibbleAccessControls.sol";
import "./NibbleStorage.sol";

contract NibbleFactory {
    uint256 public count;
    address public listenersImplementation;
    address public conditionsImplementation;
    address public agentsImplementation;
    address public storageImplementation;
    address public evaluationsImplementation;
    address public adaptersImplementation;
    address public accessControlsImplementation;
    mapping(address => NibbleLibrary.Nibble[]) private _nibbles;
    mapping(address => uint256) private _nibbleCount;

    event NibbleDeployed(
        address storageContract,
        address listenersContract,
        address conditionsContract,
        address evaluationsContract,
        address agentsContract,
        address adaptersContract,
        address accessControlsContract,
        bytes id,
        uint256 count
    );

    constructor(
        address storageImp,
        address listenersImp,
        address conditionsImp,
        address agentsImp,
        address evaluationsImp,
        address adaptersImp,
        address accessControlsImp
    ) {
        listenersImplementation = listenersImp;
        conditionsImplementation = conditionsImp;
        agentsImplementation = agentsImp;
        storageImplementation = storageImp;
        evaluationsImplementation = evaluationsImp;
        adaptersImplementation = adaptersImp;
        accessControlsImplementation = accessControlsImp;
        count = 0;
    }

    function deployFromFactory()
        external
        returns (address[7] memory, bytes memory, uint256)
    {
        address _newStorage = Clones.clone(storageImplementation);
        address _newConditions = Clones.clone(conditionsImplementation);
        address _newListeners = Clones.clone(listenersImplementation);
        address _newAdapters = Clones.clone(adaptersImplementation);
        address _newAgents = Clones.clone(agentsImplementation);
        address _newEvaluations = Clones.clone(evaluationsImplementation);
        address _newAccessControls = Clones.clone(accessControlsImplementation);

        NibbleAccessControls(_newAccessControls).initialize(msg.sender);
        NibbleStorage(_newStorage).initialize();
        NibbleConditions(_newConditions).initialize();
        NibbleListeners(_newListeners).initialize();
        NibbleAgents(_newAgents).initialize();
        NibbleEvaluations(_newEvaluations).initialize();
        NibbleAdapters(_newAdapters).initialize();

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
                adapterContract: _newAdapters,
                accessControlContract: _newAccessControls,
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
            _newAdapters,
            _newAccessControls,
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
                _newAdapters,
                _newAccessControls
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

    function getNibbleAdapterContract(
        address deployer,
        uint256 index
    ) public view returns (address) {
        return _nibbles[deployer][index].adapterContract;
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
