// SPDX-License-Identifier: MIT

pragma solidity 0.6.12;

import "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import "@openzeppelin/contracts/token/ERC721/ERC721Burnable.sol";

/*
    Mock contract used in the testing of tagging NFT assets
*/
contract ERC721BurnableMock is ERC721("MockERC721", "MK721"), ERC721Burnable {
    uint256 tokenPointer = 0;

    function mint() public {
        tokenPointer = tokenPointer + 1;
        _mint(msg.sender, tokenPointer);
    }
}
