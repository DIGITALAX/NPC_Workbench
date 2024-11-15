// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import "./NibbleLibrary.sol";
import "./NibbleAccessControls.sol";

contract NibbleListeners is Initializable {
    NibbleAccessControls public nibbleAccessControls;
    address public nibbleStorage;
    mapping(bytes => NibbleLibrary.Listener) private _allListeners;

    event ListenersModified(
        bytes[] listenerId,
        string[] metadata,
        bool[] encrypted,
        bool[] newListener
    );
    event ListenersRemoved(bytes[] listenerId);

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

    function addOrModifyListenersBatch(
        NibbleLibrary.Listener[] memory listeners
    ) external onlyWriterOrStorage {
        bool[] memory _newListeners = new bool[](listeners.length);
        bool[] memory _encrypteds = new bool[](listeners.length);
        string[] memory _metadatas = new string[](listeners.length);
        bytes[] memory _ids = new bytes[](listeners.length);

        for (uint256 i = 0; i < listeners.length; i++) {
            if (_allListeners[listeners[i].id].id.length == 0) {
                _newListeners[i] = true;
            } else {
                _newListeners[i] = false;
            }

            _allListeners[listeners[i].id] = NibbleLibrary.Listener({
                id: listeners[i].id,
                metadata: listeners[i].metadata,
                encrypted: listeners[i].encrypted
            });

            _encrypteds[i] = listeners[i].encrypted;
            _metadatas[i] = listeners[i].metadata;
            _ids[i] = listeners[i].id;
        }

        emit ListenersModified(_ids, _metadatas, _encrypteds, _newListeners);
    }

    function removeListenersBatch(
        bytes[] memory listenerIds
    ) external onlyWriterOrStorage {
        for (uint256 i = 0; i < listenerIds.length; i++) {
            delete _allListeners[listenerIds[i]];
        }

        emit ListenersRemoved(listenerIds);
    }

    function getListenerIsEncrypted(
        bytes memory listenerId
    ) public view returns (bool) {
        return _allListeners[listenerId].encrypted;
    }

    function getListenerMetadata(
        bytes memory listenerId
    ) public view returns (string memory) {
        return _allListeners[listenerId].metadata;
    }
}
