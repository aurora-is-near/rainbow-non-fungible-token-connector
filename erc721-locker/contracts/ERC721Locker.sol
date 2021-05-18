// SPDX-License-Identifier: MIT

pragma solidity 0.8.4;

import { IERC721 } from "@openzeppelin/contracts/token/ERC721/IERC721.sol";
import { Strings } from "@openzeppelin/contracts/utils/Strings.sol";
import { Borsh } from "rainbow-bridge/contracts/eth/nearbridge/contracts/Borsh.sol";
import { AdminControlled } from "rainbow-bridge/contracts/eth/nearbridge/contracts/AdminControlled.sol";
import { ProofDecoder } from "rainbow-bridge/contracts/eth/nearprover/contracts/ProofDecoder.sol";

import { IERC721Locker } from "./interfaces/IERC721Locker.sol";
import { INearProver, Locker } from "./Locker.sol";

// todo add admin controlled extension
contract ERC721Locker is IERC721Locker, Locker, AdminControlled {
    using Strings for uint256;

    event ERC721Locked (
        address indexed token,
        address indexed owner,
        uint256 indexed tokenId,
        string accountId
    );

    event Unlocked (
        address token,
        uint256 tokenId,
        address recipient
    );

    // Function output from burning non-fungible token on Near side.
    struct BurnResult {
        string tokenId; // Near NFT token ID is a string
        address token;
        address recipient;
    }

    // NFT contract address -> string token ID -> uint256
    mapping(address => string => uint256) public stringTokenIdToUnitForNft;

    constructor(bytes memory _nearTokenFactory, INearProver _nearProver, address _lockerAdmin) AdminControlled() {

        require(_lockerAdmin != address(0), "Invalid locker admin");
        require(address(_nearProver) != address(0), "Invalid near prover");
        require(_nearTokenFactory.length > 0, "Invalid near token factory");



        nearTokenFactory_ = _nearTokenFactory;
        prover_ = _nearProver;
    }

    function lockToken(address _token, uint256 _tokenId, string calldata _nearRecipientAccountId) external override {
        require(_token != address(0), "lockToken: Token cannot be address zero");
        stringTokenIdToUnitForNft[_token][_tokenId.toString()] = _tokenId;
        IERC721(_token).transferFrom(msg.sender, address(this), _tokenId);
        emit ERC721Locked(_token, msg.sender, _tokenId, _nearRecipientAccountId);
    }

    function unlockToken(bytes calldata _proofData, uint64 _proofBlockHeader) external override {
        ProofDecoder.ExecutionStatus memory status = _parseProof(_proofData, _proofBlockHeader);
        BurnResult memory result = _decodeBurnResult(status.successValue);

        uint256 tokenId = stringTokenIdToUnitForNft[result.token][result.tokenId];

        IERC721(result.token).safeTransferFrom(address(this), result.recipient, tokenId);

        emit Unlocked(result.token, tokenId, result.recipient);
    }

    function _decodeBurnResult(bytes memory data) internal pure returns(BurnResult memory result) {
        Borsh.Data memory borshData = Borsh.from(data);

        uint8 flag = borshData.decodeU8();
        require(flag == 0, "ERR_NOT_WITHDRAW_RESULT");

        return BurnResult({
            tokenId: string(bytes32ToString(borshData.decodeBytes32())), //todo: change to decodeBytes as 32 bytes would only be a 16 char string...
            token: address(uint160(borshData.decodeBytes20())),
            recipient: address(uint160(borshData.decodeBytes20()))
        });
    }

    function bytes32ToString(bytes32 _bytes32) private pure returns (string memory) {
        uint8 i = 0;
        while(i < 32 && _bytes32[i] != 0) {
            i++;
        }
        bytes memory bytesArray = new bytes(i);
        for (i = 0; i < 32 && _bytes32[i] != 0; i++) {
            bytesArray[i] = _bytes32[i];
        }
        return string(bytesArray);
    }
}
