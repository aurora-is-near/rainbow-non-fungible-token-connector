//SPDX-License-Identifier: Unlicense
pragma solidity 0.8.7;

import "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721Burnable.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721Pausable.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721Enumerable.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721URIStorage.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "./interfaces/INFTFactory.sol";

contract BridgedNFT is
    Ownable,
    ERC721Enumerable,
    ERC721Burnable,
    ERC721Pausable,
    ERC721URIStorage
{
    // Token name
    string private _name;

    // Token symbol
    string private _symbol;

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

    constructor(
        string memory _nearAccount,
        address _nftFactory,
        address _owner,
        string memory name_,
        string memory symbol_
    ) ERC721("", "") Ownable() {
        nearAccount = _nearAccount;
        nftFactory = _nftFactory;
        _name = name_;
        _symbol = symbol_;
        _transferOwnership(_owner);
    }

    function name() public view override returns (string memory) {
        return _name;
    }

    function symbol() public view override returns (string memory) {
        return _symbol;
    }

    /// @notice This function should only be called from the nft factory, it allows to mint a
    /// new nft token.
    /// @param _tokenId nft token id.
    /// @param _recipient owner of the nft.
    /// @param _tokenUri token uri.
    function mintNFT(
        uint256 _tokenId,
        address _recipient,
        string memory _tokenUri
    ) external {
        require(msg.sender == nftFactory, "Caller is not the nft factory");
        _safeMint(_recipient, _tokenId);
        _setTokenURI(_tokenId, _tokenUri);
    }

    /// @notice This function allows to start the process of unlock the token from near side,
    /// by burning the nft token.
    /// @param _tokenId nft token id.
    function withdrawNFT(uint256 _tokenId, string memory _recipientNearAccount)
        external
    {
        require(
            !INFTFactory(nftFactory).pauseBridgedWithdraw(),
            "Withdrawal is disabled"
        );
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

    function _burn(uint256 tokenId)
        internal
        virtual
        override(ERC721, ERC721URIStorage)
    {
        super._burn(tokenId);
    }

    function tokenURI(uint256 tokenId)
        public
        view
        virtual
        override(ERC721, ERC721URIStorage)
        returns (string memory)
    {
        return super.tokenURI(tokenId);
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

    function setMetadata(string memory name_, string memory symbol_) external {
        require(msg.sender == nftFactory, "Caller is not the nft factory");
        _name = name_;
        _symbol = symbol_;
    }
}
