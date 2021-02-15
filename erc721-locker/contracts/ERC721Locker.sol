// SPDX-License-Identifier: MIT

pragma solidity 0.6.12;

import "@openzeppelin/contracts/token/ERC721/IERC721.sol";
import "rainbow-bridge/contracts/eth/nearbridge/contracts/Borsh.sol";
import "rainbow-bridge/contracts/eth/nearprover/contracts/ProofDecoder.sol";

import { IERC721Locker } from "./interfaces/IERC721Locker.sol";
import { INearProver, Locker } from "./Locker.sol";
import { StringToUint256 } from "./StringToUint256.sol";

// todo: proxy once tests have covered logic
contract ERC721Locker is IERC721Locker, Locker {
    using StringToUint256 for string;

    event LockedForNativeNear (
        address indexed token,
        address indexed sender,
        uint256 indexed tokenId,
        string accountId
    );

    event LockedForNearEVM (
        address indexed token,
        address indexed sender,
        uint256 indexed tokenId,
        address nearEvmAddress,
        uint256 migrationFee
    );

    event Unlocked (
        address token,
        uint256 tokenId,
        address recipient
    );

    // Function output from burning non-fungible token on Near side.
    struct BurnResult {
        string tokenId; // comes back as string as Near side cannot handle uint256
        address token;
        address recipient;
    }

    function init(bytes memory _nearTokenFactory, INearProver _nearProver) external {
        require(address(prover_) == address(0), "Can only call init() once");
        require(address(_nearProver) != address(0), "Invalid near prover");
        require(_nearTokenFactory.length > 0, "Invalid near token factory");

        nearTokenFactory_ = _nearTokenFactory;
        prover_ = _nearProver;
    }

    function lockToken(address _token, uint256 _tokenId, string calldata _nearRecipientAccountId) external override {
        require(_token != address(0), "lockToken: Token cannot be address zero");
        IERC721(_token).transferFrom(msg.sender, address(this), _tokenId);
        emit LockedForNativeNear(_token, msg.sender, _tokenId, _nearRecipientAccountId);
    }

    function lockToken(address _token, uint256 _tokenId, address _nearEvmAddress, uint256 _migrationFee) external override {
        require(_token != address(0), "lockToken: Token cannot be address zero");
        require(_nearEvmAddress != address(0), "lockToken: Recipient evm address cannot be address zero");
        IERC721(_token).transferFrom(msg.sender, address(this), _tokenId);
        emit LockedForNearEVM(_token, msg.sender, _tokenId, _nearEvmAddress, _migrationFee);
    }

    function unlockToken(bytes calldata _proofData, uint64 _proofBlockHeader) external override {
        ProofDecoder.ExecutionStatus memory status = _parseProof(_proofData, _proofBlockHeader);
        BurnResult memory result = _decodeBurnResult(status.successValue);

        uint256 tokenId = result.tokenId.toUint256();

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
