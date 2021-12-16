//SPDX-License-Identifier: Unlicense
pragma solidity 0.8.7;

interface INFTFactory {
    function pauseBridgedWithdraw() external returns (bool);
}
