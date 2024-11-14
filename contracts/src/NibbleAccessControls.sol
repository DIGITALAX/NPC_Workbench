// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import "./NibbleLibrary.sol";

contract NibbleAccessControls is Initializable {
    address private _admin;
    bool public initialized;
    address public nibbleFactory;

    event AdminChanged(address indexed oldAdmin, address indexed newAdmin);
    event RoleGranted(address indexed agent, string role);
    event RoleRevoked(address indexed agent, string role);

    modifier OnlyNibbleFactory() {
        if (msg.sender != nibbleFactory) {
            revert NibbleLibrary.InvalidInitializer();
        }
        _;
    }

    constructor(address nibbleFactoryAddress) {
        nibbleFactory = nibbleFactoryAddress;
    }

    function initialize(address admin) external OnlyNibbleFactory {
        if (initialized) {
            revert NibbleLibrary.AlreadyInitialized();
        }
        _admin = admin;
        initialized = true;
    }

    function getAdmin() public view returns (address) {
        return _admin;
    }
}
