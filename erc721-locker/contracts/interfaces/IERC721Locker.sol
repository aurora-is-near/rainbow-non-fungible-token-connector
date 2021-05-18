// SPDX-License-Identifier: MIT

pragma solidity 0.8.4;

interface IERC721Locker {
    function lockToken(address _token, uint256 _tokenId, string calldata _nearRecipientAccountId) external virtual;
    function unlockToken(bytes calldata _proofData, uint64 _proofBlockHeader) external virtual;
}
