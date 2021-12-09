//SPDX-License-Identifier: Unlicense
pragma solidity 0.8.7;

import "hardhat/console.sol";
import "@openzeppelin/contracts-upgradeable/token/ERC721/ERC721Upgradeable.sol";

contract BridgedNFT is ERC721Upgradeable {
    /// @notice near account id ie: "NFT"
    string public nearAccount;

    /// @notice the bridge factory address.
    address public bridgeFactory;

    /// @notice Withdraw event.
    event Withdraw(
        address tokenAddress,
        address sender,
        string tokenAccountId,
        uint256 tokenId,
        string recipient
    );

    constructor(string memory _nearAccount, address _bridgeFactory) {
        __ERC721_init("", "");
        nearAccount = _nearAccount;
        bridgeFactory = _bridgeFactory;
    }

    /// @notice This function should only be called from the factory, it allows to mint a
    /// new nft token
    /// @dev check if the token id not exists, then mint a new one by calling _mint function
    /// inherited from the ERC721Upgradeable then pass _recipient and _tokenId.
    /// @param _tokenId nft token id.
    /// @param _recipient owner of the nft.
    function mintNFT(uint256 _tokenId, address _recipient) external {}

    /// @notice This function allows to start the process of unlock the token from near side,
    /// by burning the nft token.
    /// @dev Burn the token, then emit an event
    /// @param _tokenId nft token id.
    function withdrawNFT(uint256 _tokenId, string memory _recipientNearAccount)
        external
    {
        // emit Withdraw event
        emit Withdraw(
            address(this),
            msg.sender,
            nearAccount,
            _tokenId,
            _recipientNearAccount
        );
    }
}
