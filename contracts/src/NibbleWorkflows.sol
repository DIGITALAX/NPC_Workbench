// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import "./NibbleLibrary.sol";
import "./NibbleAccessControls.sol";

contract NibbleWorkflows is Initializable {
    NibbleAccessControls public nibbleAccessControls;
    address public nibbleStorage;
    mapping(bytes => NibbleLibrary.Workflow) private _allWorkflows;

    event WorkflowModified(
        bytes workflowId,
        string metadata,
        bool encrypted,
        bool newWorkflow
    );
    event WorkflowRemoved(bytes workflowId);

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

    function addOrModifyWorkflow(
        NibbleLibrary.Workflow memory workflow
    ) external onlyWriterOrStorage {
        bool _newWorkflow = true;

        if (_allWorkflows[workflow.id].id.length == 0) {
            _newWorkflow = true;
        }

        _allWorkflows[workflow.id] = NibbleLibrary.Workflow({
            id: workflow.id,
            metadata: workflow.metadata,
            encrypted: workflow.encrypted
        });

        emit WorkflowModified(
            workflow.id,
            workflow.metadata,
            workflow.encrypted,
            _newWorkflow
        );
    }

    function removeWorkflow(
        bytes memory workflowId
    ) external onlyWriterOrStorage {
        delete _allWorkflows[workflowId];

        emit WorkflowRemoved(workflowId);
    }

    function getWorkflowIsEncrypted(
        bytes memory workflowId
    ) public view returns (bool) {
        return _allWorkflows[workflowId].encrypted;
    }

    function getWorkflowMetadata(
        bytes memory workflowId
    ) public view returns (string memory) {
        return _allWorkflows[workflowId].metadata;
    }
}
