// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import "./NibbleLibrary.sol";
import "./NibbleAccessControls.sol";

contract NibbleFHEGates is Initializable {
    NibbleAccessControls public nibbleAccessControls;
    address public nibbleStorage;
    mapping(bytes => NibbleLibrary.FHEGate) private _allFHEGates;

    event FHEGatesModified(
        bytes[] fheGateId,
        string[] metadata,
        bool[] encrypted,
        bool[] onChain,
        bool[] newFHEGate
    );
    event FHEGatesRemoved(bytes[] fheGateId);

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

    function addOrModifyFHEGatesBatch(
        NibbleLibrary.FHEGate[] memory fheGates
    ) external onlyWriterOrStorage {
        bool[] memory _newFHEGates = new bool[](fheGates.length);
        bool[] memory _encrypteds = new bool[](fheGates.length);
        bool[] memory _onChains = new bool[](fheGates.length);
        string[] memory _metadatas = new string[](fheGates.length);
        bytes[] memory _ids = new bytes[](fheGates.length);

        for (uint256 i = 0; i < fheGates.length; i++) {
            if (_allFHEGates[fheGates[i].id].id.length == 0) {
                _newFHEGates[i] = true;
            } else {
                _newFHEGates[i] = false;
            }

            _allFHEGates[fheGates[i].id] = NibbleLibrary.FHEGate({
                id: fheGates[i].id,
                metadata: fheGates[i].metadata,
                encrypted: fheGates[i].encrypted
            });

            _encrypteds[i] = fheGates[i].encrypted;
            _metadatas[i] = fheGates[i].metadata;
            _ids[i] = fheGates[i].id;
        }

        emit FHEGatesModified(
            _ids,
            _metadatas,
            _encrypteds,
            _onChains,
            _newFHEGates
        );
    }

    function removeFHEGatesBatch(
        bytes[] memory fheGateIds
    ) external onlyWriterOrStorage {
        for (uint256 i = 0; i < fheGateIds.length; i++) {
            delete _allFHEGates[fheGateIds[i]];
        }

        emit FHEGatesRemoved(fheGateIds);
    }

    function getFHEGateIsEncrypted(
        bytes memory fheGateId
    ) public view returns (bool) {
        return _allFHEGates[fheGateId].encrypted;
    }

    function getFHEGateMetadata(
        bytes memory fheGateId
    ) public view returns (string memory) {
        return _allFHEGates[fheGateId].metadata;
    }
}
