// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import "./NibbleLibrary.sol";
import "./NibbleAccessControls.sol";

contract NibbleAgents is Initializable {
    NibbleAccessControls public nibbleAccessControls;
    address public nibbleStorage;
    mapping(bytes => NibbleLibrary.Agent) private _allAgents;

    event AgentsModified(
        bytes[] agentId,
        string[] metadata,
        bool[] encrypted,
        bool[] writers,
        bool[] newAgent
    );
    event AgentsRemoved(bytes[] agentId);

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
    ) external onlyNibbleFactory(nibbleFactoryAddress) onlyInitializing {
        nibbleAccessControls = NibbleAccessControls(
            nibbleAccessControlsAddress
        );
        nibbleStorage = nibbleStorageAddress;
    }

    function addOrModifyAgentsBatch(
        NibbleLibrary.Agent[] memory agents
    ) external onlyWriterOrStorage {
        bool[] memory _newAgents = new bool[](agents.length);
        bool[] memory _encrypteds = new bool[](agents.length);
        bool[] memory _writers = new bool[](agents.length);
        string[] memory _metadatas = new string[](agents.length);
        bytes[] memory _ids = new bytes[](agents.length);

        for (uint256 i = 0; i < agents.length; i++) {
            if (_allAgents[agents[i].id].id.length == 0) {
                _newAgents[i] = true;
            } else {
                _newAgents[i] = false;
            }

            _allAgents[agents[i].id] = NibbleLibrary.Agent({
                id: agents[i].id,
                metadata: agents[i].metadata,
                encrypted: agents[i].encrypted,
                writer: agents[i].writer,
                wallet: agents[i].wallet
            });

            _encrypteds[i] = agents[i].encrypted;
            _metadatas[i] = agents[i].metadata;
            _ids[i] = agents[i].id;
        }

        emit AgentsModified(
            _ids,
            _metadatas,
            _encrypteds,
            _writers,
            _newAgents
        );
    }

    function removeAgentsBatch(
        bytes[] memory agentIds
    ) external onlyWriterOrStorage {
        for (uint256 i = 0; i < agentIds.length; i++) {
            delete _allAgents[agentIds[i]];
        }

        emit AgentsRemoved(agentIds);
    }

    function getAgentIsEncrypted(
        bytes memory agentId
    ) public view returns (bool) {
        return _allAgents[agentId].encrypted;
    }

    function getAgentMetadata(
        bytes memory agentId
    ) public view returns (string memory) {
        return _allAgents[agentId].metadata;
    }

    function getAgentWallet(
        bytes memory agentId
    ) public view returns (address) {
        return _allAgents[agentId].wallet;
    }

    function getAgentWriter(bytes memory agentId) public view returns (bool) {
        return _allAgents[agentId].writer;
    }
}
