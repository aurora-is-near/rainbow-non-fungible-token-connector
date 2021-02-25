const { BN, constants, expectEvent, expectRevert } = require('@openzeppelin/test-helpers');
const { ZERO_ADDRESS } = constants;

const { expect } = require('chai');

const ERC721LockerABI = require('../artifacts/contracts/ERC721Locker.sol/ERC721Locker.json').abi
const ERC721Locker = artifacts.require('ERC721Locker')
const ERC721LockerMock = artifacts.require('ERC721LockerMock')
const TransparentUpgradeableProxyMock = artifacts.require('TransparentUpgradeableProxyMock')

const ERC721BurnableMock = artifacts.require('ERC721BurnableMock')

contract('ERC721 Locker with TransparentUpgradeableProxy Tests', function ([deployer, random, nearProver, tokenOwner, proxyAdmin, lockerAdmin, ...otherAccounts]) {
  const TOKEN_1_ID = new BN('1')

  beforeEach(async () => {
    // this will be what the proxy points to
    this.lockerLogic = await ERC721Locker.new()

    // deploys the proxy and calls init on the implementation
    this.proxy = await TransparentUpgradeableProxyMock.new(
      this.lockerLogic.address,
      proxyAdmin,
      await new web3.eth.Contract(ERC721LockerABI).methods.init(
        Buffer.from('nft.factory.near'),
        nearProver,
        lockerAdmin
      ).encodeABI(),
      {from: deployer}
    )
  })

  describe('Proxy correctly delegates calls', () => {
    beforeEach(async () => {
      this.proxy = await ERC721Locker.at(this.proxy.address)

      // deploy a mock token and mint the first NFT
      this.mockToken = await ERC721BurnableMock.new()
      await this.mockToken.mint({from: tokenOwner})

      // approve the proxy i.e. locker
      await this.mockToken.approve(this.proxy.address, TOKEN_1_ID, {from: tokenOwner})
    })

    it('Can lock a token for a given near recipient', async () => {
      const {receipt} = await this.proxy.lockToken(
        this.mockToken.address,
        TOKEN_1_ID,
        "mynearaccount.near",
        {from: tokenOwner}
      )

      await expectEvent(receipt, 'LockedForNativeNear', {
        token: this.mockToken.address,
        sender: tokenOwner,
        tokenId: TOKEN_1_ID,
        accountId: "mynearaccount.near"
      })

      expect(await this.mockToken.ownerOf(TOKEN_1_ID)).to.be.equal(this.proxy.address)
    })
  })

  describe('upgradeTo()', () => {
    it('Can update the implementation of the proxy', async () => {
      this.newERC721Locker = await ERC721LockerMock.new()

      await this.proxy.upgradeTo(
        this.newERC721Locker.address,
        {from: proxyAdmin}
      )

      const proxyToNewLocker = await ERC721LockerMock.at(this.proxy.address)

      expect(await proxyToNewLocker.thisWillReturnFalse()).to.be.false
    })
  })

  describe('upgradeToAndCall()', () => {
    it('When upgrading can call init on the target implementation', async () => {
      this.newERC721Locker = await ERC721LockerMock.new()

      // update impl and call init
      await this.proxy.upgradeToAndCall(
        this.newERC721Locker.address,
        await new web3.eth.Contract(ERC721LockerABI).methods.init(
          Buffer.from('nft.factory.near'),
          nearProver,
          lockerAdmin
        ).encodeABI(),
        {from: proxyAdmin}
      )

      // check that init has been called by trying to call again
      const locker = await ERC721Locker.at(this.proxy.address)
      await expectRevert(
        locker.init(
          Buffer.from('nft.factory.near'),
          nearProver,
          lockerAdmin,
          {from: random}
        ),
        "Can only call init() once per version"
      )
    })
  })
})
