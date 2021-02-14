// SPDX-License-Identifier: MIT

pragma solidity 0.6.12;

import "@openzeppelin/contracts/math/SafeMath.sol";

library StringToUint256 {
    using SafeMath for uint256;

    // todo: does this handle if string is not a number? should revert if not.
    // todo: safe math should cope with overflow
    function toUint256(string memory s) internal pure returns (uint256 result) {
        bytes memory b = bytes(s);
        uint i;
        result = 0;
        for (i = 0; i < b.length; i++) {
            uint c = uint(uint8(b[i]));
            if (c >= 48 && c <= 57) {
                result = result.mul(10).add(c.sub(48));
            }
        }
    }
}
