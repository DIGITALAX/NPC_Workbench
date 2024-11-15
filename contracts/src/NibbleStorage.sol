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

contract NibbleStorage is Initializable {
    NibbleAccessControls public nibbleAccessControls;
    NibbleConditions public nibbleConditions;
    NibbleListeners public nibbleListeners;
    NibbleConnectors public nibbleConnectors;
    NibbleAgents public nibbleAgents;
    NibbleEvaluations public nibbleEvaluations;

    event AdaptersModified();
    event AdaptersDeleted();

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
        address nibbleEvaluationsAddress
    ) external onlyNibbleFactory(nibbleFactoryAddress) onlyInitializing {
        nibbleConditions = NibbleConditions(nibbleConditionsAddress);
        nibbleListeners = NibbleListeners(nibbleListenersAddress);
        nibbleConnectors = NibbleConnectors(nibbleConnectorsAddress);
        nibbleAgents = NibbleAgents(nibbleAgentsAddress);
        nibbleEvaluations = NibbleEvaluations(nibbleEvaluationsAddress);
        nibbleAccessControls = NibbleAccessControls(
            nibbleAccessControlsAddress
        );
    }

    function addOrModifyAdaptersBatch(
        NibbleLibrary.ModifyAdapters memory adapters
    ) external {
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

        emit AdaptersModified();
    }

    function removeAdaptersBatch(
        NibbleLibrary.RemoveAdapters memory adapters
    ) external {
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

        emit AdaptersDeleted();
    }
}
