// SPDX-License-Identifier: MIT

pragma solidity 0.6.12;

import "@openzeppelin/contracts/math/SafeMath.sol";

library StringToUint256 {
    using SafeMath for uint256;

    /// @dev should revert if string is not a number
    /// @dev safe math should cope with overflow
    function toUint256(string memory s) internal pure returns (uint256 result) {
        bytes memory bytesOfString = bytes(s);
        require(bytesOfString.length <= 78, "Number in string could cause overflow due to length");

        result = 0;

        for (uint i = 0; i < bytesOfString.length; i++) {
            uint charCode = uint(uint8(bytesOfString[i]));
            if (charCode >= 48 && charCode <= 57) {
                result = result.mul(10).add(charCode.sub(48));
            } else {
                revert("String is not a number");
            }
        }
    }
}

// https://github.com/provable-things/ethereum-api/blob/master/lib-experimental/oraclizeAPI_lib.sol
