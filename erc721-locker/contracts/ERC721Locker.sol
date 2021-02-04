pragma solidity 0.6.12;

import "@openzeppelin/contracts/token/ERC721/IERC721.sol";
import "rainbow-bridge/contracts/eth/nearbridge/contracts/Borsh.sol";
import "rainbow-bridge/contracts/eth/nearprover/contracts/ProofDecoder.sol";

import { IERC721Locker } from "./interfaces/IERC721Locker.sol";
import { INearProver, Locker } from "./Locker.sol";
import { StringToUint256 } from "./StringToUint256.sol";

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

    constructor(bytes memory _nearTokenFactory, INearProver _nearProver) public {
        nearTokenFactory_ = _nearTokenFactory;
        prover_ = _nearProver;
    }

    function lockToken(address _token, uint256 _tokenId, string calldata _nearRecipientAccountId) external override {
        IERC721(_token).transferFrom(msg.sender, address(this), _tokenId);
        emit LockedForNativeNear(_token, msg.sender, _tokenId, _nearRecipientAccountId);
    }

    function lockToken(address _token, uint256 _tokenId, address _nearEvmAddress, uint256 _migrationFee) external override {
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
            tokenId: string(borshData.decodeBytes()),
            token: address(uint160(borshData.decodeBytes20())),
            recipient: address(uint160(borshData.decodeBytes20()))
        });
    }
}
