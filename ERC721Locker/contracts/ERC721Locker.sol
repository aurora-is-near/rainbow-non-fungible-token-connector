// SPDX-License-Identifier: MIT

pragma solidity 0.8.7;

import "@openzeppelin/contracts-upgradeable/token/ERC721/IERC721Upgradeable.sol";
import "@openzeppelin/contracts-upgradeable/token/ERC721/extensions/IERC721MetadataUpgradeable.sol";
import "@openzeppelin/contracts-upgradeable/token/ERC721/IERC721ReceiverUpgradeable.sol";
import "@openzeppelin/contracts-upgradeable/utils/introspection/IERC165Upgradeable.sol";
import "@openzeppelin/contracts-upgradeable/utils/StringsUpgradeable.sol";
import "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";

import "rainbow-bridge/contracts/eth/nearbridge/contracts/AdminControlled.sol";
import "rainbow-bridge/contracts/eth/nearprover/contracts/ProofDecoder.sol";

import "./interfaces/IERC721Locker.sol";
import "./Locker.sol";

contract ERC721Locker is
    IERC721Locker,
    IERC721ReceiverUpgradeable,
    UUPSUpgradeable,
    Locker,
    AdminControlled
{
    using StringsUpgradeable for uint256;

    event Locked(
        address indexed token,
        address indexed sender,
        string tokenId,
        string accountId,
        string tokenUri
    );

    event Unlocked(address token, uint256 tokenId, address recipient);
    /*
     *     bytes4(keccak256('name()')) == 0x06fdde03
     *     bytes4(keccak256('symbol()')) == 0x95d89b41
     *     bytes4(keccak256('tokenURI(uint256)')) == 0xc87b56dd
     *
     *     => 0x06fdde03 ^ 0x95d89b41 ^ 0xc87b56dd == 0x5b5e139f
     */
    bytes4 private constant _INTERFACE_ID_ERC721_METADATA = 0x5b5e139f;
    uint8 constant PAUSE_FINALISE_FROM_NEAR = 1 << 0;
    uint8 constant PAUSE_TRANSFER_TO_NEAR = 1 << 1;

    function initialize(
        bytes memory _nearTokenFactory,
        INearProver _nearProver,
        uint64 _minBlockAcceptanceHeight,
        uint256 _pausedFlags
    ) public initializer {
        __UUPSUpgradeable_init();
        __AdminControlled_init(_pausedFlags);
        __Locker_init(
            _nearTokenFactory,
            _nearProver,
            _minBlockAcceptanceHeight
        );
    }

    function migrateMultipleTokensToNear(
        address _token,
        uint256[] memory _tokenIds,
        string memory _nearRecipientAccountId
    ) external override {
        for (uint256 i = 0; i < _tokenIds.length; i++) {
            migrateTokenToNear(_token, _tokenIds[i], _nearRecipientAccountId);
        }
    }

    function migrateTokenToNear(
        address _token,
        uint256 _tokenId,
        string memory _nearRecipientAccountId
    ) public override pausable(PAUSE_TRANSFER_TO_NEAR) {
        string memory tokenIdAsString = _tokenId.toString();

        IERC721Upgradeable(_token).safeTransferFrom(
            msg.sender,
            address(this),
            _tokenId
        );

        string memory tokenURI = "";
        if (
            IERC165Upgradeable(_token).supportsInterface(
                _INTERFACE_ID_ERC721_METADATA
            )
        ) {
            tokenURI = IERC721MetadataUpgradeable(_token).tokenURI(_tokenId);
        }

        emit Locked(
            _token,
            msg.sender,
            tokenIdAsString,
            _nearRecipientAccountId,
            tokenURI
        );
    }

    function finishNearToEthMigration(
        bytes calldata _proofData,
        uint64 _proofBlockHeader
    ) external override pausable(PAUSE_FINALISE_FROM_NEAR) {
        ProofDecoder.ExecutionStatus memory status = _parseAndConsumeProof(
            _proofData,
            _proofBlockHeader
        );

        Borsh.Data memory borshDataFromProof = Borsh.from(status.successValue);

        uint8 flag = Borsh.decodeU8(borshDataFromProof);
        condition(flag == 0, "ERR_NOT_WITHDRAW_RESULT");

        address nftAddress = address(
            uint160(Borsh.decodeBytes20(borshDataFromProof))
        );
        address recipient = address(
            uint160(Borsh.decodeBytes20(borshDataFromProof))
        );
        string memory tokenIdAsString = string(
            Borsh.decodeBytes(borshDataFromProof)
        );

        uint256 tokenId = stringToUint(tokenIdAsString);
        IERC721Upgradeable(nftAddress).safeTransferFrom(
            address(this),
            recipient,
            tokenId
        );

        emit Unlocked(nftAddress, tokenId, recipient);
    }

    function setNearTokenFactory(bytes memory _nearTokenFactory)
        external
        onlyRole(DEFAULT_ADMIN_ROLE)
    {
        nearTokenFactory = _nearTokenFactory;
    }

    function setNearProver(INearProver _nearProver)
        external
        onlyRole(DEFAULT_ADMIN_ROLE)
    {
        nearProver = _nearProver;
    }

    function setMinBlockAcceptanceHeight(uint64 _minBlockAcceptanceHeight)
        external
        onlyRole(DEFAULT_ADMIN_ROLE)
    {
        minBlockAcceptanceHeight = _minBlockAcceptanceHeight;
    }

    function _authorizeUpgrade(address newImplementation)
        internal
        override
        onlyRole(DEFAULT_ADMIN_ROLE)
    {}

    /// @notice Implement @openzeppelin/contracts/token/ERC721/IERC721Receiver.sol interface.
    function onERC721Received(
        address,
        address,
        uint256,
        bytes calldata
    ) external pure override returns (bytes4) {
        return
            bytes4(
                keccak256("onERC721Received(address,address,uint256,bytes)")
            );
    }

    function stringToUint(string memory s) public pure returns (uint256) {
        bytes memory b = bytes(s);
        uint256 result = 0;
        uint256 oldResult = 0;
        for (uint256 i = 0; i < b.length; i++) {
            if (uint8(b[i]) >= 48 && uint8(b[i]) <= 57) {
                oldResult = result;
                result = result * 10 + (uint256(uint8(b[i])) - 48);
                if (oldResult > result) {
                    revert("Invalid number");
                }
            } else {
                revert("Invalid number");
            }
        }
        return result;
    }
}
