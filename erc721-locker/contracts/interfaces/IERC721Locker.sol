// SPDX-License-Identifier: MIT

pragma solidity 0.8.7;

interface IERC721Locker {
    function migrateMultipleTokensToNear(address _token, uint256[] calldata _tokenIds, string calldata _nearRecipientAccountId) external;
    function migrateTokenToNear(address _token, uint256 _tokenId, string calldata _nearRecipientAccountId) external;
    function finishNearToEthMigration(bytes calldata _proofData, uint64 _proofBlockHeader) external;
}
