const { BN, constants, expectEvent, expectRevert } = require('@openzeppelin/test-helpers');
const { ZERO_ADDRESS } = constants;

const { expect } = require('chai');

const NearAccessControls = artifacts.require('NearAccessControls')

contract('NearAccessControls Contract tests', function ([deployer, roleRecipient, ...otherAccounts]) {
  beforeEach(async () => {
    this.accessControls = await NearAccessControls.new({from: deployer})
  })

  describe('addAdmin()', () => {
    it('Grants admin role from an admin account', async () => {
      const { receipt } = await this.accessControls.addAdmin(roleRecipient, {from: deployer})
      await expectEvent(receipt, 'AdminRoleGranted', {
        beneficiary: roleRecipient
      })

      expect(await this.accessControls.isAdmin(roleRecipient)).to.be.true
    })

    it('Reverts when not admin', async () => {
      await expectRevert(
        this.accessControls.addAdmin(roleRecipient, {from: roleRecipient}),
        "AccessControls: Only admin"
      )
    })
  })

  describe('removeAdmin()', () => {
    it('Revokes admin role from a sender admin account', async () => {
      await this.accessControls.addAdmin(roleRecipient, {from: deployer})
      expect(await this.accessControls.isAdmin(roleRecipient)).to.be.true

      const { receipt } = await this.accessControls.removeAdmin(roleRecipient, {from: deployer})
      await expectEvent(receipt, 'AdminRoleRemoved', {
        beneficiary: roleRecipient,
      })

      expect(await this.accessControls.isAdmin(roleRecipient)).to.be.false
    })

    it('Reverts when not admin', async () => {
      await expectRevert(
        this.accessControls.removeAdmin(roleRecipient, {from: roleRecipient}),
        "AccessControls: Only admin"
      )
    })
  })

  describe('addWhitelist()', () => {
    it('Grants admin role from an admin account', async () => {
      const { receipt } = await this.accessControls.addWhitelist(roleRecipient, {from: deployer})
      await expectEvent(receipt, 'WhitelistRoleGranted', {
        beneficiary: roleRecipient
      })

      expect(await this.accessControls.isWhitelisted(roleRecipient)).to.be.true
    })

    it('Reverts when not admin', async () => {
      await expectRevert(
        this.accessControls.addWhitelist(roleRecipient, {from: roleRecipient}),
        "AccessControls: Only admin"
      )
    })
  })

  describe('removeWhitelist()', () => {
    it('Revokes admin role from a sender admin account', async () => {
      await this.accessControls.addWhitelist(roleRecipient, {from: deployer})
      expect(await this.accessControls.isWhitelisted(roleRecipient)).to.be.true

      const { receipt } = await this.accessControls.removeWhitelist(roleRecipient, {from: deployer})
      await expectEvent(receipt, 'WhitelistRoleRemoved', {
        beneficiary: roleRecipient,
      })

      expect(await this.accessControls.isWhitelisted(roleRecipient)).to.be.false
    })

    it('Reverts when not admin', async () => {
      await expectRevert(
        this.accessControls.removeWhitelist(roleRecipient, {from: roleRecipient}),
        "AccessControls: Only admin"
      )
    })
  })

  describe('addSmartContract()', () => {
    it('Grants admin role from an admin account', async () => {
      const { receipt } = await this.accessControls.addSmartContract(roleRecipient, {from: deployer})
      await expectEvent(receipt, 'SmartContractRoleGranted', {
        beneficiary: roleRecipient
      })

      expect(await this.accessControls.isSmartContract(roleRecipient)).to.be.true
    })

    it('Reverts when not admin', async () => {
      await expectRevert(
        this.accessControls.addSmartContract(roleRecipient, {from: roleRecipient}),
        "AccessControls: Only admin"
      )
    })
  })

  describe('removeSmartContract()', () => {
    it('Revokes admin role from a sender admin account', async () => {
      await this.accessControls.addSmartContract(roleRecipient, {from: deployer})
      expect(await this.accessControls.isSmartContract(roleRecipient)).to.be.true

      const { receipt } = await this.accessControls.removeSmartContract(roleRecipient, {from: deployer})
      await expectEvent(receipt, 'SmartContractRoleRemoved', {
        beneficiary: roleRecipient,
      })

      expect(await this.accessControls.isSmartContract(roleRecipient)).to.be.false
    })

    it('Reverts when not admin', async () => {
      await expectRevert(
        this.accessControls.removeSmartContract(roleRecipient, {from: roleRecipient}),
        "AccessControls: Only admin"
      )
    })
  })
})
