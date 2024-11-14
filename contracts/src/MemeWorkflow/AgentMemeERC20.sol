// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";

contract AgentMemeERC20 is ERC20, Initializable {
    // uint256 public tokenSupply;
    // uint256 public totalMinted;
    // string private _tokenName;
    // string private _tokenSymbol;
    //     bool public initialized;

    // error InvalidAgentRole();
    // error ExceedsTotalSupply();
    // error AlreadyInitialized();

    // modifier OnlyAgentOrAdmin() {
    //     if (
    //         msg.sender != accessControls.getAdmin() &&
    //         !accessControls.hasAgentTokenDistributorPermissions(msg.sender)
    //     ) {
    //         revert InvalidAgentRole();
    //     }
    //     _;
    // }

    // function initialize(
    //     string memory tokenName,
    //     string memory tokenSymbol,
    //     address accessControlsAddress,
    //     uint256 supply
    // ) external {
    //     if (initialized) {
    //         revert AlreadyInitialized();
    //     }
    //     _tokenName = tokenName;
    //     _tokenSymbol = tokenSymbol;
    //     tokenSupply = supply;
    //     initialized = true;
    //     accessControls = AgentMemeAccessControls(accessControlsAddress);
    // }

    constructor() ERC20("", "") {}

    // function mintandDistributeToken(
    //     address to,
    //     uint256 amount
    // ) external OnlyAgentOrAdmin {
    //     if (totalMinted + amount > tokenSupply) {
    //         revert ExceedsTotalSupply();
    //     }
    //     totalMinted += amount;
    //     _mint(to, amount);
    // }

    // function name() public view override returns (string memory) {
    //     return _tokenName;
    // }

    // function symbol() public view override returns (string memory) {
    //     return _tokenSymbol;
    // }
}
