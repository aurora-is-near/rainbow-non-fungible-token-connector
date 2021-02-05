// SPDX-License-Identifier: MIT

pragma solidity 0.6.12;

import { StringToUint256 } from "../StringToUint256.sol";

contract StringToUint256Tester {
    using StringToUint256 for string;

    function convert(string calldata _str) external pure returns (uint256) {
        return _str.toUint256();
    }
}
