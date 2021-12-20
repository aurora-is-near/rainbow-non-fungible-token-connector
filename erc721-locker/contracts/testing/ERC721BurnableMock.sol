// SPDX-License-Identifier: MIT

pragma solidity 0.8.7;

// import "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import "@openzeppelin/contracts-upgradeable/token/ERC721/ERC721Upgradeable.sol";

/*
    Mock contract used in the testing of tagging NFT assets
*/
contract ERC721BurnableMock is ERC721Upgradeable {
    uint256 tokenPointer = 0;

    function mint() public {
        tokenPointer = tokenPointer + 1;
        _mint(msg.sender, tokenPointer);
    }
}
