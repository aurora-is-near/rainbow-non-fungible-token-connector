//SPDX-License-Identifier: Unlicense
pragma solidity 0.8.7;

import "rainbow-bridge/contracts/eth/nearprover/contracts/ProofDecoder.sol";
import "rainbow-bridge/contracts/eth/nearprover/contracts/INearProver.sol";
import "rainbow-bridge/contracts/eth/nearbridge/contracts/AdminControlled.sol";
import "./BridgedNFT.sol";

contract NFTFactory is AdminControlled {
    using Borsh for Borsh.Data;
    using ProofDecoder for Borsh.Data;

    event ConsumedProof(bytes32 indexed _receiptId);

    /// @notice this mapping stores the near contract name with the evm contract copy address.
    mapping(string => address) public bridgedNFTs;

    /// @notice the near prover address.
    INearProver public nearProver;

    /// @notice the near prover address.
    bytes public nearLocker;

    /// @notice the near prover address.
    uint64 public minBlockAcceptanceHeight;

    /// @notice OutcomeReciptId -> Used.
    mapping(bytes32 => bool) public usedEvents;

    /// @notice pause the finaliseNearToEthTransfer function.
    uint8 public constant PAUSE_FINALISE_FROM_NEAR = 1 << 0;

    /// @notice pause all the bridged withdraw process.
    bool public pauseBridgedWithdraw;

    function initialize(
        INearProver _nearProver,
        bytes memory _nearLocker,
        uint64 _minBlockAcceptanceHeight,
        uint256 _flags
    ) external initializer {
        __AdminControlled_init(_flags);
        nearProver = _nearProver;
        minBlockAcceptanceHeight = _minBlockAcceptanceHeight;
        nearLocker = _nearLocker;
    }

    /// @notice This function allows to finalise the bridge process by calling the
    /// evm contract and mint the new token.
    /// @param _proofData near proof.
    /// @param _proofBlockHeader proof block header.
    function finaliseNearToEthTransfer(
        bytes calldata _proofData,
        uint64 _proofBlockHeader
    ) external pausable(PAUSE_FINALISE_FROM_NEAR) {
        ProofDecoder.ExecutionStatus memory status = _parseAndConsumeProof(
            _proofData,
            _proofBlockHeader
        );

        Borsh.Data memory borshDataFromProof = Borsh.from(status.successValue);

        uint8 flag = Borsh.decodeU8(borshDataFromProof);
        require(flag == 0, "ERR_NOT_LOCK_RESULT");

        address recipient = address(
            uint160(Borsh.decodeBytes20(borshDataFromProof))
        );

        string memory accountID = string(Borsh.decodeBytes(borshDataFromProof));

        string memory tokenIdAsString = string(
            Borsh.decodeBytes(borshDataFromProof)
        );
        uint256 tokenId = stringToUint(tokenIdAsString);

        // mint new nft
        address erc721Address = bridgedNFTs[accountID];
        require(erc721Address != address(0), "Contract not deployed");
        BridgedNFT(erc721Address).mintNFT(tokenId, recipient);
    }

    function _parseAndConsumeProof(
        bytes memory proofData,
        uint64 proofBlockHeight
    ) internal returns (ProofDecoder.ExecutionStatus memory result) {
        require(
            nearProver.proveOutcome(proofData, proofBlockHeight),
            "Proof should be valid"
        );

        // Unpack the proof and extract the execution outcome.
        Borsh.Data memory borshData = Borsh.from(proofData);
        ProofDecoder.FullOutcomeProof memory fullOutcomeProof = borshData
            .decodeFullOutcomeProof();

        require(
            fullOutcomeProof.block_header_lite.inner_lite.height >=
                minBlockAcceptanceHeight,
            "Proof is from the ancient block"
        );

        bytes32 receiptId = fullOutcomeProof
            .outcome_proof
            .outcome_with_id
            .outcome
            .receipt_ids[0];
        require(!usedEvents[receiptId], "The lock event cannot be reused");
        usedEvents[receiptId] = true;

        require(
            keccak256(
                fullOutcomeProof
                    .outcome_proof
                    .outcome_with_id
                    .outcome
                    .executor_id
            ) == keccak256(nearLocker),
            "Can only mint tokens from the linked mintable not fungible token on Near blockchain."
        );

        result = fullOutcomeProof.outcome_proof.outcome_with_id.outcome.status;
        require(
            !result.failed,
            "Cannot use failed execution outcome for minting the tokens."
        );
        require(
            !result.unknown,
            "Cannot use unknown execution outcome for minting the tokens."
        );

        emit ConsumedProof(receiptId);
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

    /// @notice Deploy a new Bridged Bridged contract.
    /// @param _nearAccount the nft near account id.
    function deployBridgedToken(
        string calldata _nearAccount,
        string memory _name,
        string memory _symbol
    ) external {
        require(
            bridgedNFTs[_nearAccount] == address(0),
            "Contract already deployed"
        );
        address tokenAddress = address(
            new BridgedNFT(_nearAccount, address(this), admin, _name, _symbol)
        );
        bridgedNFTs[_nearAccount] = tokenAddress;
    }

    /// @notice Set pause bridged withdraw.
    /// @param _paused pause the withdraw process.
    function setPauseBridgedWithdraw(bool _paused) external {
        pauseBridgedWithdraw = _paused;
    }
}
