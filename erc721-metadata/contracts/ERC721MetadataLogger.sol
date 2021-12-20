// SPDX-License-Identifier: GPL-3.0

pragma solidity ^0.8.5;

import "@openzeppelin/contracts/token/ERC721/extensions/IERC721Metadata.sol";

/**
 * @title ERC721MetadataLogger
 * @dev emits and retreive ERC-721 metadata
 */
contract ERC721MetadataLogger {

    event Log(
            address indexed erc721,
            string name,
            string symbol,
            uint256 timestamp
        );

    /**
     * @dev log values from the erc721 contract
     * @param erc721 contract address
     */
    function log(address erc721) external {
        IERC721Metadata _erc721 = IERC721Metadata(erc721);
        emit Log(
                erc721,
                _erc721.name(),
                _erc721.symbol(),
                block.number
            );
    }
}
