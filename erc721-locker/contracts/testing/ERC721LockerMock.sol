// SPDX-License-Identifier: MIT

pragma solidity 0.8.4;

import { ERC721Locker } from "../ERC721Locker.sol";

contract ERC721LockerMock is ERC721Locker {

    function version() override internal pure returns (uint256) {
        return 2;
    }

    function thisWillReturnFalse() external pure returns (bool) {
        return false;
    }
}
