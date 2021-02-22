// SPDX-License-Identifier: MIT

pragma solidity 0.6.12;

import "@openzeppelin/contracts/access/AccessControl.sol";

// todo: should this be moved out this repo if it's generic to any Near contract on ETH?
contract NearAccessControls is AccessControl {
    bytes32 public constant WHITELIST_ROLE = keccak256("WHITELIST");
    bytes32 public constant SMART_CONTRACT_ROLE = keccak256("SMART_CONTRACT");

    event AdminRoleGranted(address indexed beneficiary);
    event AdminRoleRemoved(address indexed beneficiary);

    event WhitelistRoleGranted(address indexed beneficiary);
    event WhitelistRoleRemoved(address indexed beneficiary);

    event SmartContractRoleGranted(address indexed beneficiary);
    event SmartContractRoleRemoved(address indexed beneficiary);

    constructor() public {
        _setupRole(DEFAULT_ADMIN_ROLE, _msgSender());
    }

    function isAdmin(address _address) external view returns (bool) {
        return hasRole(DEFAULT_ADMIN_ROLE, _address);
    }

    function isWhitelisted(address _address) external view returns (bool) {
        return hasRole(WHITELIST_ROLE, _address);
    }

    function isSmartContract(address _address) external view returns (bool) {
        return hasRole(SMART_CONTRACT_ROLE, _address);
    }

    function addAdmin(address _address) external {
        require(hasRole(DEFAULT_ADMIN_ROLE, _msgSender()), "AccessControls: Only admin");
        _setupRole(DEFAULT_ADMIN_ROLE, _address);
        emit AdminRoleGranted(_address);
    }

    function removeAdmin(address _address) external {
        require(hasRole(DEFAULT_ADMIN_ROLE, _msgSender()), "AccessControls: Only admin");
        revokeRole(DEFAULT_ADMIN_ROLE, _address);
        emit AdminRoleRemoved(_address);
    }

    function addWhitelist(address _address) external {
        require(hasRole(DEFAULT_ADMIN_ROLE, _msgSender()), "AccessControls: Only admin");
        _setupRole(WHITELIST_ROLE, _address);
        emit WhitelistRoleGranted(_address);
    }

    function removeWhitelist(address _address) external {
        require(hasRole(DEFAULT_ADMIN_ROLE, _msgSender()), "AccessControls: Only admin");
        revokeRole(WHITELIST_ROLE, _address);
        emit WhitelistRoleRemoved(_address);
    }

    function addSmartContract(address _address) external {
        require(hasRole(DEFAULT_ADMIN_ROLE, _msgSender()), "AccessControls: Only admin");
        _setupRole(SMART_CONTRACT_ROLE, _address);
        emit SmartContractRoleGranted(_address);
    }

    function removeSmartContract(address _address) external {
        require(hasRole(DEFAULT_ADMIN_ROLE, _msgSender()), "AccessControls: Only admin");
        revokeRole(SMART_CONTRACT_ROLE, _address);
        emit SmartContractRoleRemoved(_address);
    }
}
