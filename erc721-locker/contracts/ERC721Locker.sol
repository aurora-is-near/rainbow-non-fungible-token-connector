// SPDX-License-Identifier: MIT

pragma solidity 0.6.12;

import { IERC721 } from "@openzeppelin/contracts/token/ERC721/IERC721.sol";
import { Strings } from "@openzeppelin/contracts/utils/Strings.sol";
import { Borsh } from "rainbow-bridge/contracts/eth/nearbridge/contracts/Borsh.sol";
import { AdminControlled } from "rainbow-bridge/contracts/eth/nearbridge/contracts/AdminControlled.sol";
import { ProofDecoder } from "rainbow-bridge/contracts/eth/nearprover/contracts/ProofDecoder.sol";

import { IERC721Locker } from "./interfaces/IERC721Locker.sol";
import { INearProver, Locker } from "./Locker.sol";

contract ERC721Locker is IERC721Locker, Locker, AdminControlled {
    using Strings for uint256;

    // TODO create pause flags and use in modifiers
    // uint constant PAUSE_FINALISE_FROM_NEAR = 1 << 0;
    // uint constant PAUSE_TRANSFER_TO_NEAR = 1 << 1;

    event Locked (
        address indexed token,
        address indexed sender,
        string tokenId,
        string accountId
    );

    event Unlocked (
        address token,
        uint256 tokenId,
        address recipient
    );

    // This reverse lookup is needed as token IDs on Near are strings and therefore this is needed for unlocking
    // NFT contract address -> string token ID -> uint256
    mapping(address => mapping(string => uint256)) public stringTokenIdToUnitForNft;

    constructor(
        bytes memory _nearTokenFactory,
        INearProver _nearProver,
        uint64 _minBlockAcceptanceHeight,
        address _admin,
        uint256 _pausedFlags
    ) AdminControlled(_admin, _pausedFlags) public {

        require(address(_nearProver) != address(0), "Invalid near prover");
        require(_nearTokenFactory.length > 0, "Invalid near token factory");

        minBlockAcceptanceHeight = _minBlockAcceptanceHeight;
        nearTokenFactory = _nearTokenFactory;
        prover = _nearProver;
    }

    function migrateMultipleTokensToNear(address _token, uint256[] calldata _tokenIds, string calldata _nearRecipientAccountId) external override {
        for(uint i = 0; i < _tokenIds.length; i++) {
            migrateTokenToNear(_token, _tokenIds[i], _nearRecipientAccountId);
        }
    }

    function migrateTokenToNear(address _token, uint256 _tokenId, string memory _nearRecipientAccountId) public override {
        string memory tokenIdAsString = _tokenId.toString();

        stringTokenIdToUnitForNft[_token][tokenIdAsString] = _tokenId;
        IERC721(_token).transferFrom(msg.sender, address(this), _tokenId);

        emit Locked(_token, msg.sender, tokenIdAsString, _nearRecipientAccountId);
    }

    function finishNearToEthMigration(bytes calldata _proofData, uint64 _proofBlockHeader) external override {
        ProofDecoder.ExecutionStatus memory status = _parseAndConsumeProof(_proofData, _proofBlockHeader);

        Borsh.Data memory borshDataFromProof = Borsh.from(status.successValue);

        uint8 flag = borshDataFromProof.decodeU8();
        require(flag == 0, "ERR_NOT_WITHDRAW_RESULT");

        address nftAddress = address(uint160(borshDataFromProof.decodeBytes20()));
        address recipient = address(uint160(borshDataFromProof.decodeBytes20()));
        string memory tokenIdAsString = string(borshDataFromProof.decodeBytes());

        uint256 tokenId = stringTokenIdToUnitForNft[nftAddress][tokenIdAsString];

        IERC721(nftAddress).safeTransferFrom(address(this), recipient, tokenId);

        emit Unlocked(nftAddress, tokenId, recipient);
    }
}
