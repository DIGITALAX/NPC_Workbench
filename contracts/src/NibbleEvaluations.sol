// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import "./NibbleLibrary.sol";
import "./NibbleAccessControls.sol";

contract NibbleEvaluations is Initializable {
    NibbleAccessControls public nibbleAccessControls;
    address public nibbleStorage;
    mapping(bytes => NibbleLibrary.Evaluation) private _allEvaluations;

    event EvaluationsModified(
        bytes[] evaluationId,
        string[] metadata,
        bool[] encrypted,
        bool[] newEvaluation
    );
    event EvaluationsRemoved(bytes[] evaluationId);

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

    function addOrModifyEvaluationsBatch(
        NibbleLibrary.Evaluation[] memory evaluations
    ) external onlyWriterOrStorage {
        bool[] memory _newEvaluations = new bool[](evaluations.length);
        bool[] memory _encrypteds = new bool[](evaluations.length);
        string[] memory _metadatas = new string[](evaluations.length);
        bytes[] memory _ids = new bytes[](evaluations.length);

        for (uint256 i = 0; i < evaluations.length; i++) {
            if (_allEvaluations[evaluations[i].id].id.length == 0) {
                _newEvaluations[i] = true;
            } else {
                _newEvaluations[i] = false;
            }

            _allEvaluations[evaluations[i].id] = NibbleLibrary.Evaluation({
                id: evaluations[i].id,
                metadata: evaluations[i].metadata,
                encrypted: evaluations[i].encrypted
            });

            _encrypteds[i] = evaluations[i].encrypted;
            _metadatas[i] = evaluations[i].metadata;
            _ids[i] = evaluations[i].id;
        }

        emit EvaluationsModified(
            _ids,
            _metadatas,
            _encrypteds,
            _newEvaluations
        );
    }

    function removeEvaluationsBatch(
        bytes[] memory evaluationIds
    ) external onlyWriterOrStorage {
        for (uint256 i = 0; i < evaluationIds.length; i++) {
            delete _allEvaluations[evaluationIds[i]];
        }

        emit EvaluationsRemoved(evaluationIds);
    }

    function getEvaluationIsEncrypted(
        bytes memory evaluationId
    ) public view returns (bool) {
        return _allEvaluations[evaluationId].encrypted;
    }

    function getEvaluationMetadata(
        bytes memory evaluationId
    ) public view returns (string memory) {
        return _allEvaluations[evaluationId].metadata;
    }
}
