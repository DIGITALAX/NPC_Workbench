// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "./AgentMemeAccessControls.sol";
import "./AgentMemeFHE.sol";
import "./MemeWorkflow/AgentMemeERC20.sol";
import "./AgentMemeData.sol";
import "@openzeppelin/contracts/proxy/Clones.sol";
import "./AgentMemeLibrary.sol";

contract AgentMemeFactory {
    uint256 public count;
    address public accessControlImplementation;
    address public fheImplementation;
    address public dataImplementation;
    address public tokenImplementation;
    mapping(address => AgentMemeLibrary.Nibble[]) private _nibbles;
    mapping(address => uint256) private _nibbleCount;

    event AgentMemeDeployed(
        address indexed admin,
        address accessControlAddress,
        address fhe,
        address data,
        address token,
        bytes id,
        uint256 count
    );

    constructor(
        address accessControl,
        address fhe,
        address data,
        address token
    ) {
        accessControlImplementation = accessControl;
        fheImplementation = fhe;
        dataImplementation = data;
        tokenImplementation = token;
        count = 0;
    }

    function deployFromFactory(
        address[] memory agentWriters,
        address[] memory agentReaders,
        address[] memory agentAdmins,
        address[] memory agentTokenDistributors,
        string memory tokenName,
        string memory tokenSymbol,
        uint256 tokenSupply
    ) external returns (address[4] memory, bytes memory, uint256) {
        address _newAC = Clones.clone(accessControlImplementation);
        address _newFHE = Clones.clone(fheImplementation);
        address _newData = Clones.clone(dataImplementation);
        address _newToken = Clones.clone(tokenImplementation);

        AgentMemeAccessControls(_newAC).initialize(
            msg.sender,
            agentWriters,
            agentReaders,
            agentAdmins,
            agentTokenDistributors
        );
        AgentMemeFHE(_newFHE).initialize(_newAC);
        address[] memory agentAddresses = _combineAgentAddresses(
            agentWriters,
            agentReaders,
            agentAdmins,
            agentTokenDistributors
        );
        AgentMemeData(_newData).initialize(agentAddresses);
        AgentMemeERC20(_newToken).initialize(
            tokenName,
            tokenSymbol,
            _newAC,
            tokenSupply
        );

        count++;
        bytes memory _id = _generateRandomId(
            msg.sender,
            block.timestamp,
            count
        );

        _nibbleCount[msg.sender] += 1;

        _nibbles[msg.sender].push(
            AgentMemeLibrary.Nibble({
                contracts: [_newAC, _newFHE, _newData, _newToken],
                id: _id,
                count: count
            })
        );

        emit AgentMemeDeployed(
            msg.sender,
            _newAC,
            _newFHE,
            _newData,
            _newToken,
            _id,
            count
        );

        return ([_newAC, _newFHE, _newData, _newToken], _id, count);
    }

    function _generateRandomId(
        address sender,
        uint256 timestamp,
        uint256 _count
    ) internal pure returns (bytes memory) {
        return
            abi.encodePacked(
                keccak256(abi.encodePacked(sender, timestamp, _count))
            );
    }

    function _combineAgentAddresses(
        address[] memory agentWriters,
        address[] memory agentReaders,
        address[] memory agentAdmins,
        address[] memory agentTokenDistributors
    ) internal pure returns (address[] memory) {
        uint256 totalLength = agentWriters.length +
            agentReaders.length +
            agentAdmins.length +
            agentTokenDistributors.length;
        address[] memory agentAddresses = new address[](totalLength);

        uint256 index = 0;
        for (uint256 i = 0; i < agentWriters.length; i++) {
            agentAddresses[index++] = agentWriters[i];
        }
        for (uint256 j = 0; j < agentReaders.length; j++) {
            agentAddresses[index++] = agentReaders[j];
        }
        for (uint256 k = 0; k < agentAdmins.length; k++) {
            agentAddresses[index++] = agentAdmins[k];
        }
        for (uint256 l = 0; l < agentTokenDistributors.length; l++) {
            agentAddresses[index++] = agentTokenDistributors[l];
        }

        return agentAddresses;
    }

    function getDeployerNibbleCount(
        address deployer
    ) public view returns (uint256) {
        return _nibbleCount[deployer];
    }

    function getNibbleContracts(
        address deployer,
        uint256 index
    ) public view returns (address[4] memory) {
        return _nibbles[deployer][index].contracts;
    }

    function getNibbleCount(
        address deployer,
        uint256 index
    ) public view returns (uint256) {
        return _nibbles[deployer][index].count;
    }

    function getNibbleId(
        address deployer,
        uint256 index
    ) public view returns (bytes memory) {
        return _nibbles[deployer][index].id;
    }
}
