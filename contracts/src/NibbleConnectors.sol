// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import "./NibbleLibrary.sol";
import "./NibbleAccessControls.sol";

contract NibbleConnectors is Initializable {
    NibbleAccessControls public nibbleAccessControls;
    address public nibbleStorage;
    mapping(bytes => NibbleLibrary.Connector) private _allConnectors;

    event ConnectorsModified(
        bytes[] connectorId,
        string[] metadata,
        bool[] encrypted,
        bool[] onChain,
        bool[] newConnector
    );
    event ConnectorsRemoved(bytes[] connectorId);

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

    function addOrModifyConnectorsBatch(
        NibbleLibrary.Connector[] memory connectors
    ) external onlyWriterOrStorage {
        bool[] memory _newConnectors = new bool[](connectors.length);
        bool[] memory _encrypteds = new bool[](connectors.length);
        bool[] memory _onChains = new bool[](connectors.length);
        string[] memory _metadatas = new string[](connectors.length);
        bytes[] memory _ids = new bytes[](connectors.length);

        for (uint256 i = 0; i < connectors.length; i++) {
            if (_allConnectors[connectors[i].id].id.length == 0) {
                _newConnectors[i] = true;
            } else {
                _newConnectors[i] = false;
            }

            _allConnectors[connectors[i].id] = NibbleLibrary.Connector({
                id: connectors[i].id,
                metadata: connectors[i].metadata,
                encrypted: connectors[i].encrypted,
                onChain: connectors[i].onChain
            });

            _encrypteds[i] = connectors[i].encrypted;
            _metadatas[i] = connectors[i].metadata;
            _ids[i] = connectors[i].id;
        }

        emit ConnectorsModified(
            _ids,
            _metadatas,
            _encrypteds,
            _onChains,
            _newConnectors
        );
    }

    function removeConnectorsBatch(
        bytes[] memory connectorIds
    ) external onlyWriterOrStorage {
        for (uint256 i = 0; i < connectorIds.length; i++) {
            delete _allConnectors[connectorIds[i]];
        }

        emit ConnectorsRemoved(connectorIds);
    }

    function getConnectorIsEncrypted(
        bytes memory connectorId
    ) public view returns (bool) {
        return _allConnectors[connectorId].encrypted;
    }

    function getConnectorMetadata(
        bytes memory connectorId
    ) public view returns (string memory) {
        return _allConnectors[connectorId].metadata;
    }

    function getConnectorOnChain(
        bytes memory connectorId
    ) public view returns (bool) {
        return _allConnectors[connectorId].onChain;
    }
}
