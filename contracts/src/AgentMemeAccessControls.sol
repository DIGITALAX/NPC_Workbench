// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";

contract AgentMemeAccessControls is Initializable {
    address private _admin;
    bool public initialized;
    mapping(address => bool) private _agentWriters;
    mapping(address => bool) private _agentAdmins;
    mapping(address => bool) private _humanWriters;
    mapping(address => bool) private _agentReaders;
    mapping(address => bool) private _agentTokenDistributors;

    event AdminChanged(address indexed oldAdmin, address indexed newAdmin);
    event RoleGranted(address indexed agent, string role);
    event RoleRevoked(address indexed agent, string role);

    error OnlyAdmin();
    error AdminCantBeZero();
    error AlreadyInitialized();

    modifier onlyAdmin() {
        if (msg.sender != _admin) {
            revert OnlyAdmin();
        }
        _;
    }

    modifier onlyAdminOrAgentAdmin() {
        if (msg.sender != _admin && !_agentAdmins[msg.sender]) {
            revert OnlyAdmin();
        }
        _;
    }

    function initialize(
        address admin,
        address[] memory agentWriters,
        address[] memory agentReaders,
        address[] memory agentAdmins,
        address[] memory agentTokenDistributors
    ) external {
        if (initialized) {
            revert AlreadyInitialized();
        }
        _admin = admin;
        initialized = true;

        for (uint256 i = 0; i < agentWriters.length; i++) {
            _agentWriters[agentWriters[i]] = true;
            emit RoleGranted(agentWriters[i], "agentWriter");
        }

        for (uint256 i = 0; i < agentReaders.length; i++) {
            _agentWriters[agentReaders[i]] = true;
            emit RoleGranted(agentReaders[i], "agentReader");
        }

        for (uint256 i = 0; i < agentAdmins.length; i++) {
            _agentAdmins[agentAdmins[i]] = true;
            emit RoleGranted(agentAdmins[i], "agentAdmin");
        }

        for (uint256 i = 0; i < agentTokenDistributors.length; i++) {
            _agentTokenDistributors[agentTokenDistributors[i]] = true;
            emit RoleGranted(
                agentTokenDistributors[i],
                "agentTokenDistributor"
            );
        }
    }

    function addAgentAdmin(address _agentAdmin) external onlyAdmin {
        _agentAdmins[_agentAdmin] = true;
        emit RoleGranted(_agentAdmin, "agentAdmin");
    }

    function removeAgentAdmin(address _agentAdmin) external onlyAdmin {
        _agentAdmins[_agentAdmin] = false;
        emit RoleRevoked(_agentAdmin, "agentAdmin");
    }

    function addAgentWriter(address _writer) external onlyAdminOrAgentAdmin {
        _agentWriters[_writer] = true;
        emit RoleGranted(_writer, "agentWriter");
    }

    function removeAgentWriter(address _writer) external onlyAdminOrAgentAdmin {
        _agentWriters[_writer] = false;
        emit RoleRevoked(_writer, "agentWriter");
    }

    function addAgentTokenDistributor(
        address _tokenDistributor
    ) external onlyAdminOrAgentAdmin {
        _agentTokenDistributors[_tokenDistributor] = true;
        emit RoleGranted(_tokenDistributor, "agentTokenDistributor");
    }

    function removeAgentTokenDistributor(
        address _tokenDistributor
    ) external onlyAdminOrAgentAdmin {
        _agentTokenDistributors[_tokenDistributor] = false;
        emit RoleRevoked(_tokenDistributor, "agentTokenDistributor");
    }

    function addHumanWriter(address _writer) external onlyAdmin {
        _humanWriters[_writer] = true;
        emit RoleGranted(_writer, "humanWriter");
    }

    function removeHumanWriter(address _writer) external onlyAdmin {
        _humanWriters[_writer] = false;
        emit RoleRevoked(_writer, "humanWriter");
    }

    function addAgentReader(address _reader) external onlyAdminOrAgentAdmin {
        _agentReaders[_reader] = true;
        emit RoleGranted(_reader, "agentReader");
    }

    function removeAgentReader(address _reader) external onlyAdminOrAgentAdmin {
        _agentReaders[_reader] = false;
        emit RoleRevoked(_reader, "agentReader");
    }

    function changeAdmin(address newAdmin) external onlyAdmin {
        if (newAdmin == address(0)) {
            revert AdminCantBeZero();
        }
        emit AdminChanged(_admin, newAdmin);
        _admin = newAdmin;
    }

    function hasAgentReadPermissions(
        address agentAddress
    ) public view returns (bool) {
        return _agentReaders[agentAddress];
    }

    function hasAgentWritePermissions(
        address agentAddress
    ) public view returns (bool) {
        return _agentWriters[agentAddress];
    }

    function hasAgentAdminPermissions(
        address agentAddress
    ) public view returns (bool) {
        return _agentAdmins[agentAddress];
    }

    function hasHumanWritePermissions(
        address humanAddress
    ) public view returns (bool) {
        return _humanWriters[humanAddress];
    }

    function hasAgentTokenDistributorPermissions(
        address agentAddress
    ) public view returns (bool) {
        return _agentTokenDistributors[agentAddress];
    }

    function getAdmin() public view returns (address) {
        return _admin;
    }
}
