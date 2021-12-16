//SPDX-License-Identifier: Unlicense
pragma solidity 0.8.7;

import "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721Burnable.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721Pausable.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721Enumerable.sol";
import "./interfaces/INFTFactory.sol";

contract BridgedNFT is ERC721Enumerable, ERC721Burnable, ERC721Pausable {
    /// @notice near account id
    string public nearAccount;

    /// @notice the bridge factory address.
    address public nftFactory;

    /// @notice Withdraw event.
    event Withdraw(
        address tokenAddress,
        address sender,
        string tokenAccountId,
        uint256 tokenId,
        string recipient
    );

    constructor(string memory _nearAccount, address _nftFactory)
        ERC721("", "")
    {
        nearAccount = _nearAccount;
        nftFactory = _nftFactory;
    }

    /// @notice This function should only be called from the nft factory, it allows to mint a
    /// new nft token.
    /// @param _tokenId nft token id.
    /// @param _recipient owner of the nft.
    function mintNFT(uint256 _tokenId, address _recipient) external {
        require(msg.sender == nftFactory, "Caller is not the nft factory");
        _safeMint(_recipient, _tokenId);
    }

    /// @notice This function allows to start the process of unlock the token from near side,
    /// by burning the nft token.
    /// @param _tokenId nft token id.
    function withdrawNFT(uint256 _tokenId, string memory _recipientNearAccount)
        external
    {
        require(!INFTFactory(nftFactory).pauseBridgedWithdraw(), "Withdrawal is disabled");
        _burn(_tokenId);

        // emit Withdraw event
        emit Withdraw(
            address(this),
            msg.sender,
            nearAccount,
            _tokenId,
            _recipientNearAccount
        );
    }

    function _beforeTokenTransfer(
        address from,
        address to,
        uint256 tokenId
    ) internal virtual override(ERC721, ERC721Enumerable, ERC721Pausable) {
        super._beforeTokenTransfer(from, to, tokenId);
    }

    function supportsInterface(bytes4 interfaceId)
        public
        view
        virtual
        override(ERC721, ERC721Enumerable)
        returns (bool)
    {
        return super.supportsInterface(interfaceId);
    }
}
