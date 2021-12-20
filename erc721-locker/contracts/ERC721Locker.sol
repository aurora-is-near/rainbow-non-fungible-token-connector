// SPDX-License-Identifier: MIT

pragma solidity 0.8.7;

import "@openzeppelin/contracts/token/ERC721/IERC721.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/IERC721Metadata.sol";
import "@openzeppelin/contracts/token/ERC721/IERC721Receiver.sol";
import "@openzeppelin/contracts/utils/introspection/IERC165.sol";
import "@openzeppelin/contracts/utils/Strings.sol";

import "rainbow-bridge/contracts/eth/nearbridge/contracts/AdminControlled.sol";
import "rainbow-bridge/contracts/eth/nearprover/contracts/ProofDecoder.sol";

import "./interfaces/IERC721Locker.sol";
import "./Locker.sol";

contract ERC721Locker is IERC721Locker, Locker, AdminControlled {
    using Strings for uint256;

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

    constructor(
        bytes memory _nearTokenFactory,
        INearProver _nearProver,
        uint64 _minBlockAcceptanceHeight,
        address _admin,
        uint256 _pausedFlags
    )
        AdminControlled(_admin, _pausedFlags)
        Locker(_nearTokenFactory, _nearProver, _minBlockAcceptanceHeight)
    {}

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

        IERC721(_token).safeTransferFrom(msg.sender, address(this), _tokenId);

        string memory tokenURI = "";
        if (IERC165(_token).supportsInterface(_INTERFACE_ID_ERC721_METADATA)) {
            tokenURI = IERC721Metadata(_token).tokenURI(_tokenId);
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
        require(flag == 0, "ERR_NOT_WITHDRAW_RESULT");

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
        IERC721(nftAddress).safeTransferFrom(address(this), recipient, tokenId);

        emit Unlocked(nftAddress, tokenId, recipient);
    }

    /// @notice Implement @openzeppelin/contracts/token/ERC721/IERC721Receiver.sol interface.
    function onERC721Received(
        address,
        address,
        uint256,
        bytes calldata
    ) external pure returns (bytes4) {
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
