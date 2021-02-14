// SPDX-License-Identifier: MIT

pragma solidity 0.6.12;

import "@openzeppelin/contracts/proxy/UpgradeableProxy.sol";
import "@openzeppelin/contracts/utils/Address.sol";
import { NearAccessControls } from "./NearAccessControls.sol";

contract NearUpgradeableProxy is UpgradeableProxy {
    using Address for address;

    /**
     * @dev Storage slot with the access controls of the contract.
     * This is the keccak-256 hash of "eip1967.proxy.accessControls" subtracted by 1, and is
     * validated in the constructor.
     */
    bytes32 private constant _ACCESS_CONTROLS_SLOT = bytes32(uint256(keccak256("eip1967.proxy.accessControls")) - 1);

    /**
     * @dev Initializes an upgradeable proxy managed by `_nearAccessControls`, backed by the implementation at `_logic`, and
     * optionally initialized with `_data` as explained in {UpgradeableProxy-constructor}.
     */
    constructor(address _logic, address _accessControls, bytes memory _data) public payable
    UpgradeableProxy(_logic, _data) {
        _setAccessControls(_accessControls);
    }

    /**
     * @dev Upgrade the implementation of the proxy.
     *
     * NOTE: Only someone with admin role in the access controls contract
     */
    function upgradeTo(address _newImplementation) public {
        require(NearAccessControls(_nearAccessControls()).isAdmin(msg.sender), "Only an admin can update implementation");
        _upgradeTo(_newImplementation);
    }

    /**
     * @dev Upgrade the implementation of the proxy, and then call a function from the new implementation as specified
     * by `data`, which should be an encoded function call. This is useful to initialize new storage variables in the
     * proxied contract.
     *
     * NOTE: Only someone with the admin can call this function. See {ProxyAdmin-upgradeAndCall}.
     */
    function upgradeToAndCall(address _newImplementation, bytes calldata _data) external payable {
        upgradeTo(_newImplementation);
        _newImplementation.functionDelegateCall(_data);
    }

    /**
     * @dev Returns access controls contract.
     */
    function _nearAccessControls() internal view virtual returns (address nearAccessControls) {
        bytes32 slot = _ACCESS_CONTROLS_SLOT;
        // solhint-disable-next-line no-inline-assembly
        assembly {
            nearAccessControls := sload(slot)
        }
    }

    /**
     * @dev Stores a new address in the EIP1967 access controls slot.
     */
    function _setAccessControls(address _accessControls) private {
        bytes32 slot = _ACCESS_CONTROLS_SLOT;

        // solhint-disable-next-line no-inline-assembly
        assembly {
            sstore(slot, _accessControls)
        }
    }

    /**
     * @dev Makes sure the admin cannot access the fallback function. See {Proxy-_beforeFallback}.
     */
    function _beforeFallback() internal virtual override {
        require(
            NearAccessControls(_nearAccessControls()).isAdmin(msg.sender),
            "Admin cannot fallback to proxy target"
        );

        super._beforeFallback();
    }
}

// based on: https://github.com/OpenZeppelin/openzeppelin-contracts/blob/18c7efe800df6fc19554ece3b1f238e9e028a1db/contracts/proxy/TransparentUpgradeableProxy.sol
