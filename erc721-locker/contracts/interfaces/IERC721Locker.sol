// SPDX-License-Identifier: MIT

pragma solidity 0.7.6;

import "@openzeppelin/contracts/token/ERC721/IERC721.sol";

abstract contract ERC721Locker {
    constructor(bytes memory _nearTokenFactory, address _nearProver) {}
    function lockToken(IERC721 _token, uint256 _tokenId, string calldata _nearRecipientAccountId) external virtual;
    function lockToken(IERC721 _token, uint256 _tokenId, address _nearEvmAddress, uint256 _migrationFee) external virtual;
    function unlockToken(bytes calldata _proofData, uint64 _proofBlockHeader) external virtual;
    function migrateToken(IERC721 _token, uint256 _tokenId) external virtual;
}
