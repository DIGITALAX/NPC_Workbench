// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import "./NibbleLibrary.sol";
import "./NibbleAgents.sol";
import "./NibbleListeners.sol";
import "./NibbleEvaluations.sol";
import "./NibbleConnectors.sol";
import "./NibbleConditions.sol";
import "./NibbleAccessControls.sol";
import "./NibbleFHEGates.sol";
import "./NibbleWorkflows.sol";

contract NibbleStorage is Initializable {
    NibbleAccessControls public nibbleAccessControls;
    NibbleConditions public nibbleConditions;
    NibbleListeners public nibbleListeners;
    NibbleConnectors public nibbleConnectors;
    NibbleAgents public nibbleAgents;
    NibbleEvaluations public nibbleEvaluations;
    NibbleFHEGates public nibbleFHEGates;
    NibbleWorkflows public nibbleWorkflows;

    event AdaptersModified(address writer);
    event AdaptersDeleted(address writer);
    event WorkflowModified(bytes workflowId, address writer);
    event WorkflowDeleted(bytes workflowId, address writer);

    modifier onlyNibbleFactory(address nibbleFactory) {
        if (msg.sender != nibbleFactory) {
            revert NibbleLibrary.InvalidInitializer();
        }
        _;
    }

    modifier onlyWriter() {
        if (
            !nibbleAccessControls.getAgentWriter(msg.sender) ||
            !nibbleAccessControls.getHumanWriter(msg.sender)
        ) {
            revert NibbleLibrary.InvalidRole();
        }
        _;
    }

    function initialize(
        address nibbleFactoryAddress,
        address nibbleAccessControlsAddress,
        address nibbleConditionsAddress,
        address nibbleListenersAddress,
        address nibbleConnectorsAddress,
        address nibbleAgentsAddress,
        address nibbleEvaluationsAddress,
        address nibbleFHEGatesAddress,
        address nibbleWorkflowsAddress
    ) external onlyNibbleFactory(nibbleFactoryAddress) {
        if (address(nibbleAccessControls) != address(0)) {
            revert NibbleLibrary.AlreadyInitialized();
        }
        nibbleConditions = NibbleConditions(nibbleConditionsAddress);
        nibbleListeners = NibbleListeners(nibbleListenersAddress);
        nibbleConnectors = NibbleConnectors(nibbleConnectorsAddress);
        nibbleAgents = NibbleAgents(nibbleAgentsAddress);
        nibbleEvaluations = NibbleEvaluations(nibbleEvaluationsAddress);
        nibbleFHEGates = NibbleFHEGates(nibbleFHEGatesAddress);
        nibbleAccessControls = NibbleAccessControls(
            nibbleAccessControlsAddress
        );
        nibbleWorkflows = NibbleWorkflows(nibbleWorkflowsAddress);
    }

    function addOrModifyWorkflow(
        NibbleLibrary.Workflow memory workflow
    ) external onlyWriter {
        nibbleWorkflows.addOrModifyWorkflow(workflow);

        emit WorkflowModified(workflow.id, msg.sender);
    }

    function removeWorkflow(bytes memory workflowId) external onlyWriter {
        nibbleWorkflows.removeWorkflow(workflowId);

        emit WorkflowDeleted(workflowId, msg.sender);
    }

    function addOrModifyAdaptersBatch(
        NibbleLibrary.ModifyAdapters memory adapters
    ) external onlyWriter {
        if (adapters.conditions.length > 0) {
            nibbleConditions.addOrModifyConditionsBatch(adapters.conditions);
        }

        if (adapters.listeners.length > 0) {
            nibbleListeners.addOrModifyListenersBatch(adapters.listeners);
        }

        if (adapters.connectors.length > 0) {
            nibbleConnectors.addOrModifyConnectorsBatch(adapters.connectors);
        }

        if (adapters.agents.length > 0) {
            nibbleAgents.addOrModifyAgentsBatch(adapters.agents);
        }

        if (adapters.evaluations.length > 0) {
            nibbleEvaluations.addOrModifyEvaluationsBatch(adapters.evaluations);
        }

        if (adapters.fheGates.length > 0) {
            nibbleFHEGates.addOrModifyFHEGatesBatch(adapters.fheGates);
        }

        emit AdaptersModified(msg.sender);
    }

    function removeAdaptersBatch(
        NibbleLibrary.RemoveAdapters memory adapters
    ) external onlyWriter {
        if (adapters.conditions.length > 0) {
            nibbleConditions.removeConditionsBatch(adapters.conditions);
        }

        if (adapters.listeners.length > 0) {
            nibbleListeners.removeListenersBatch(adapters.listeners);
        }

        if (adapters.connectors.length > 0) {
            nibbleConnectors.removeConnectorsBatch(adapters.connectors);
        }

        if (adapters.agents.length > 0) {
            nibbleAgents.removeAgentsBatch(adapters.agents);
        }

        if (adapters.evaluations.length > 0) {
            nibbleEvaluations.removeEvaluationsBatch(adapters.evaluations);
        }

        if (adapters.fheGates.length > 0) {
            nibbleFHEGates.removeFHEGatesBatch(adapters.fheGates);
        }

        emit AdaptersDeleted(msg.sender);
    }
}
