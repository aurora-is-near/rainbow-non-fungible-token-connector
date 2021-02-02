pragma solidity 0.6.12;

import "@openzeppelin/contracts/token/ERC721/IERC721.sol";

import { IERC721Locker } from "./interfaces/IERC721Locker.sol";
import { INearProver, Locker } from "./Locker.sol";

contract ERC721Locker is IERC721Locker, Locker {
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

    }
}
