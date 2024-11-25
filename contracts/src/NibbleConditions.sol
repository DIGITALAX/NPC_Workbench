// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import "./NibbleLibrary.sol";
import "./NibbleAccessControls.sol";

contract NibbleConditions is Initializable {
    NibbleAccessControls public nibbleAccessControls;
    address public nibbleStorage;
    mapping(bytes => NibbleLibrary.Condition) private _allConditions;

    event ConditionsModified(
        bytes[] conditionId,
        string[] metadata,
        bool[] encrypted,
        bool[] newCondition
    );
    event ConditionsRemoved(bytes[] conditionId);

    modifier onlyNibbleFactory(address _nibbleFactory) {
        if (msg.sender != _nibbleFactory) {
            revert NibbleLibrary.InvalidInitializer();
        }
        _;
    }

    modifier onlyWriterOrStorage() {
        if (
            !nibbleAccessControls.getAgentWriter(msg.sender) ||
            !nibbleAccessControls.getHumanWriter(msg.sender) ||
            msg.sender != nibbleStorage
        ) {
            revert NibbleLibrary.InvalidRole();
        }
        _;
    }

    function initialize(
        address nibbleFactoryAddress,
        address nibbleAccessControlsAddress,
        address nibbleStorageAddress
    ) external onlyNibbleFactory(nibbleFactoryAddress) {
        if (address(nibbleAccessControls) != address(0)) {
            revert NibbleLibrary.AlreadyInitialized();
        }
        nibbleAccessControls = NibbleAccessControls(
            nibbleAccessControlsAddress
        );
        nibbleStorage = nibbleStorageAddress;
    }

    function addOrModifyConditionsBatch(
        NibbleLibrary.Condition[] memory conditions
    ) external onlyWriterOrStorage {
        bool[] memory _newConditions = new bool[](conditions.length);
        bool[] memory _encrypteds = new bool[](conditions.length);
        string[] memory _metadatas = new string[](conditions.length);
        bytes[] memory _ids = new bytes[](conditions.length);

        for (uint256 i = 0; i < conditions.length; i++) {
            if (_allConditions[conditions[i].id].id.length == 0) {
                _newConditions[i] = true;
            } else {
                _newConditions[i] = false;
            }

            _allConditions[conditions[i].id] = NibbleLibrary.Condition({
                id: conditions[i].id,
                metadata: conditions[i].metadata,
                encrypted: conditions[i].encrypted
            });

            _encrypteds[i] = conditions[i].encrypted;
            _metadatas[i] = conditions[i].metadata;
            _ids[i] = conditions[i].id;
        }

        emit ConditionsModified(_ids, _metadatas, _encrypteds, _newConditions);
    }

    function removeConditionsBatch(
        bytes[] memory conditionIds
    ) external onlyWriterOrStorage {
        for (uint256 i = 0; i < conditionIds.length; i++) {
            delete _allConditions[conditionIds[i]];
        }

        emit ConditionsRemoved(conditionIds);
    }

    function getConditionIsEncrypted(
        bytes memory conditionId
    ) public view returns (bool) {
        return _allConditions[conditionId].encrypted;
    }

    function getConditionMetadata(
        bytes memory conditionId
    ) public view returns (string memory) {
        return _allConditions[conditionId].metadata;
    }
}
