// SPDX-License-Identifier: MIT

pragma solidity 0.8.7;

import "rainbow-bridge/contracts/eth/nearprover/contracts/ProofDecoder.sol";
import "rainbow-bridge/contracts/eth/nearprover/contracts/INearProver.sol";
import "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";

contract Locker is Initializable {
    using Borsh for Borsh.Data;
    using ProofDecoder for Borsh.Data;

    event ConsumedProof(bytes32 indexed _receiptId);

    INearProver public nearProver;
    bytes public nearTokenFactory;

    /// Proofs from blocks that are below the acceptance height will be rejected.
    /// If `minBlockAcceptanceHeight` value is zero - proofs from block with any
    /// height are accepted.
    uint64 public minBlockAcceptanceHeight;

    // OutcomeReciptId -> Used
    mapping(bytes32 => bool) public usedEvents;

    constructor(
        bytes memory _nearTokenFactory,
        INearProver _nearProver,
        uint64 _minBlockAcceptanceHeight
    ) {
        require(
            _nearTokenFactory.length > 0,
            "Invalid Near Token Factory address"
        );
        require(
            address(_nearProver) != address(0),
            "Invalid Near prover address"
        );

        nearTokenFactory = _nearTokenFactory;
        nearProver = _nearProver;
        minBlockAcceptanceHeight = _minBlockAcceptanceHeight;
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
        require(!usedEvents[receiptId], "The burn event cannot be reused");
        usedEvents[receiptId] = true;

        require(
            keccak256(
                fullOutcomeProof
                    .outcome_proof
                    .outcome_with_id
                    .outcome
                    .executor_id
            ) == keccak256(nearTokenFactory),
            "Can only unlock tokens from the linked mintable fungible token on Near blockchain."
        );

        result = fullOutcomeProof.outcome_proof.outcome_with_id.outcome.status;
        require(
            !result.failed,
            "Cannot use failed execution outcome for unlocking the tokens."
        );
        require(
            !result.unknown,
            "Cannot use unknown execution outcome for unlocking the tokens."
        );

        emit ConsumedProof(receiptId);
    }
}
