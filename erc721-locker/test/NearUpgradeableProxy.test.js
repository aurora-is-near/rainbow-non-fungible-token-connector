const { BN, constants, expectEvent, expectRevert } = require('@openzeppelin/test-helpers');
const { ZERO_ADDRESS } = constants;

const { expect } = require('chai');

const ERC721LockerABI = require('../artifacts/contracts/ERC721Locker.sol/ERC721Locker.json').abi
const ERC721Locker = artifacts.require('ERC721Locker')
const NearAccessControls = artifacts.require('NearAccessControls')
const NearUpgradeableProxy = artifacts.require('NearUpgradeableProxy')

const ERC721BurnableMock = artifacts.require('ERC721BurnableMock')

contract('NearUpgradeableProxy Tests', function ([deployer, random, nearProver, tokenOwner, ...otherAccounts]) {
  const TOKEN_1_ID = new BN('1')

  beforeEach(async () => {
    // this will be what the proxy points to
    this.lockerLogic = await ERC721Locker.new()

    this.accessControls = await NearAccessControls.new({from: deployer})

    // deploys the proxy and calls init on the implementation
    this.proxy = await NearUpgradeableProxy.new(
      this.lockerLogic.address,
      this.accessControls.address,
      await new web3.eth.Contract(ERC721LockerABI).methods.init(
        Buffer.from('nft.factory.near'),
        nearProver
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
      await this.proxy.upgradeTo(
        this.accessControls.address,
        {from: deployer}
      )

      const accessControlsViaProxy = await NearAccessControls.at(this.proxy.address)

      // as no one is setup in contract, deployer should not be admin
      expect(await accessControlsViaProxy.isAdmin(deployer, {from: random})).to.be.false
    })
  })

  describe('upgradeToAndCall()', () => {
    it('When upgrading can call init on the target implementation', async () => {
      // create a proxy but dont call init in constructor
      this.proxy = await NearUpgradeableProxy.new(
        this.lockerLogic.address,
        this.accessControls.address,
        Buffer.from(''),
        {from: deployer}
      )

      // update impl and call init
      await this.proxy.upgradeToAndCall(
        this.lockerLogic.address,
        await new web3.eth.Contract(ERC721LockerABI).methods.init(
          Buffer.from('nft.factory.near'),
          nearProver
        ).encodeABI(),
        {from: deployer}
      )

      // check that init has been called by trying to call again
      const locker = await ERC721Locker.at(this.proxy.address)
      await expectRevert(
        locker.init(
          Buffer.from('nft.factory.near'),
          nearProver,
          {from: random}
        ),
        "Can only call init() once"
      )
    })
  })
})
