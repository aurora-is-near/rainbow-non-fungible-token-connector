const { BN, constants, expectEvent, expectRevert } = require('@openzeppelin/test-helpers');
const { ZERO_ADDRESS } = constants;

const { expect } = require('chai');

const ERC721Locker = artifacts.require('ERC721Locker')
const ERC721BurnableMock = artifacts.require('ERC721BurnableMock')

contract('ERC721Locker', function ([deployer, ...otherAccounts]) {
  const TOKEN_1_ID = new BN('1')

  beforeEach(async () => {
    // todo: constructor should not allow these 'null' value
    this.locker = await ERC721Locker.new('0x0', ZERO_ADDRESS)

    // deploy a mock token and mint the first NFT
    this.mockToken = await ERC721BurnableMock.new()
    await this.mockToken.mint()
  })

  describe('Locking for Near native', () => {
    it('Can lock a token for a given near recipient', async () => {
      // approve the locker
      await this.mockToken.approve(this.locker.address, TOKEN_1_ID)

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
  })
})
