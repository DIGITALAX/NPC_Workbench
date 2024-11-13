// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "./AgentMemeAccessControls.sol";

contract AgentMemeERC20 is ERC20 {
    uint256 public immutable tokenSupply; 
    AgentMemeAccessControls public accessControls;
    uint256 public totalMinted; 

    error InvalidAgentRole();
    error ExceedsTotalSupply();

    modifier OnlyAgentOrAdmin() {
        if (
            msg.sender != accessControls.getAdmin() &&
            !accessControls.hasAgentTokenDistributorPermissions(msg.sender)
        ) {
            revert InvalidAgentRole();
        }
        _;
    }

    constructor(
        string memory name,
        string memory symbol,
        address accessControlsAddress,
        uint256 supply
    ) ERC20(name, symbol) {
        tokenSupply = supply;
        accessControls = AgentMemeAccessControls(accessControlsAddress);
    }

    function mintandDistributeToken(
        address to,
        uint256 amount
    ) external OnlyAgentOrAdmin {
        if (totalMinted + amount > tokenSupply) {
            revert ExceedsTotalSupply();
        }
        totalMinted += amount;
        _mint(to, amount);
    }
}
