// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import "./NibbleLibrary.sol";

contract NibbleAccessControls is Initializable {
    address private _admin;
    mapping(address => bool) private _humanWriters;
    mapping(address => bool) private _agentWriters;

    event AdminChanged(address indexed oldAdmin, address indexed newAdmin);
    event RoleGranted(address indexed agent, string role);
    event RoleRevoked(address indexed agent, string role);

    modifier onlyNibbleFactory(address nibbleFactory) {
        if (msg.sender != nibbleFactory) {
            revert NibbleLibrary.InvalidInitializer();
        }
        _;
    }

    function initialize(
        address nibbleFactoryAddress,
        address admin
    ) external onlyNibbleFactory(nibbleFactoryAddress)  {
        if (_admin != address(0)) {
            revert NibbleLibrary.AlreadyInitialized();
        }
        _admin = admin;

        _humanWriters[admin] = true;
    }

    function getHumanWriter(address writer) public view returns (bool) {
        return _humanWriters[writer];
    }

    function getAgentWriter(address writer) public view returns (bool) {
        return _agentWriters[writer];
    }

    function getAdmin() public view returns (address) {
        return _admin;
    }
}
