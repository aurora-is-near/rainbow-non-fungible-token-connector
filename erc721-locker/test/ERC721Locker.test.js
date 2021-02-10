const { BN, constants, expectEvent, expectRevert } = require('@openzeppelin/test-helpers');
const { ZERO_ADDRESS } = constants;

const { expect } = require('chai');

const ERC721Locker = artifacts.require('ERC721Locker')
const ERC721BurnableMock = artifacts.require('ERC721BurnableMock')

contract('ERC721Locker', function ([deployer, nearEvmBeneficiary, ...otherAccounts]) {
  const TOKEN_1_ID = new BN('1')

  beforeEach(async () => {
    // todo: constructor should not allow these 'null' value
    this.locker = await ERC721Locker.new('0x0', ZERO_ADDRESS)

    // deploy a mock token and mint the first NFT
    this.mockToken = await ERC721BurnableMock.new()
    await this.mockToken.mint()

    // approve the locker
    await this.mockToken.approve(this.locker.address, TOKEN_1_ID)
  })

  describe('Locking for Near native', () => {
    it('Can lock a token for a given near recipient', async () => {
      const {receipt} = await this.locker.lockToken(
        this.mockToken.address,
        TOKEN_1_ID,
        "mynearaccount.near"
      )

      await expectEvent(receipt, 'LockedForNativeNear', {
        token: this.mockToken.address,
        sender: deployer,
        tokenId: TOKEN_1_ID,
        accountId: "mynearaccount.near"
      })

      expect(await this.mockToken.ownerOf(TOKEN_1_ID)).to.be.equal(this.locker.address)
    })

    it('Reverts when address zero is supplied as token address', async () => {
      await expectRevert(
        this.locker.lockToken(ZERO_ADDRESS, TOKEN_1_ID, "mynearaccount.near"),
        "lockToken: Token cannot be address zero"
      )
    })
  })

  describe('Locking for Near EVM', () => {
    it('Can lock a token for a given EVM recipient', async () => {
      const migrationFee = '55';

      const {receipt} = await this.locker.lockToken(
        this.mockToken.address,
        TOKEN_1_ID,
        nearEvmBeneficiary,
        migrationFee
      )

      await expectEvent(receipt, 'LockedForNearEVM', {
        token: this.mockToken.address,
        sender: deployer,
        tokenId: TOKEN_1_ID,
        nearEvmAddress: nearEvmBeneficiary,
        migrationFee: migrationFee
      })

      expect(await this.mockToken.ownerOf(TOKEN_1_ID)).to.be.equal(this.locker.address)
    })

    it('Reverts when address zero is supplied as token address', async () => {
      await expectRevert(
        this.locker.lockToken(
          ZERO_ADDRESS,
          TOKEN_1_ID,
          nearEvmBeneficiary,
          '0'
        ),
        "lockToken: Token cannot be address zero"
      )
    })

    it('Reverts when address zero is supplied as EVM address', async () => {
      await expectRevert(
        this.locker.lockToken(
          this.mockToken.address,
          TOKEN_1_ID,
          ZERO_ADDRESS,
          '0'
        ),
        "lockToken: Recipient evm address cannot be address zero"
      )
    })
  })
})
