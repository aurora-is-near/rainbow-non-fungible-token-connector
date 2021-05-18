pragma solidity 0.8.4;

import "rainbow-bridge/contracts/eth/nearprover/contracts/INearProver.sol";

contract NearProverMock is INearProver {
    function proveOutcome(bytes memory proofData, uint64 blockHeight) override public view returns(bool) {
        return true;
    }
}
