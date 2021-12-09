//SPDX-License-Identifier: Unlicense
pragma solidity 0.8.7;

import "hardhat/console.sol";

contract NFTFactory {
    /// @notice this mapping stores the near contract name with the evm contract copy address.
    /// ie: NFT => 0x123456
    mapping(string => address) public bridgedNFTs;

    /// @notice the near prover address.
    address public prover;

    constructor(address _prover) {
        prover = _prover;
    }

    /// @notice This function allows to finalise the bridge process by calling the
    /// evm contract and mint the new token.
    /// @dev ***DON'T DO THIS ONE FOR NOW***.
    /// @param _proofData near proof.
    function finaliseNearToEthTransfer(bytes calldata _proofData) external {}

    /// @notice Deploy a new Bridged (ERC721) contract.
    /// @dev before deploying the contract we have to check if the contract was already
    /// deployed, if not we deploy a new BridgedNFT contract and store his address inside
    /// bridgedNFTs mapping.
    /// @param _nearAccount the near account name ie: "NFT"
    function deployBridgedToken(string calldata _nearAccount) external {}
}
