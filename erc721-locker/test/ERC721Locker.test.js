const { BN, constants, expectEvent, expectRevert } = require('@openzeppelin/test-helpers');
const { ZERO_ADDRESS } = constants;

const { expect } = require('chai');

const { serialize } = require('rainbow-bridge-lib/rainbow/borsh.js');
const { borshifyOutcomeProof } = require('rainbow-bridge-lib/rainbow/borshify-proof.js');

const { toWei, fromWei, hexToBytes } = web3.utils;

const ERC721Locker = artifacts.require('ERC721Locker')
const NearProverMock = artifacts.require('test/NearProverMock')
const ERC721BurnableMock = artifacts.require('ERC721BurnableMock')

// todo change for ERC721 locker event
const SCHEMA = {
  'Unlock': {
    kind: 'struct', fields: [
      ['flag', 'u8'],
      ['tokenId', [32]],
      ['token', [20]],
      ['recipient', [20]],
    ]
  }
};

contract('ERC721Locker', function ([deployer, nearProver, nearEvmBeneficiary, unlockBeneficiary, lockerAdmin, ...otherAccounts]) {
  const TOKEN_1_ID = new BN('1')

  beforeEach(async () => {
    this.prover = await NearProverMock.new();
    this.locker = await ERC721Locker.new()
    await this.locker.init(
      Buffer.from('nearnonfuntoken', 'utf-8'),
      this.prover.address,
      lockerAdmin
    )

    // deploy a mock token and mint the first NFT
    this.mockToken = await ERC721BurnableMock.new()
    await this.mockToken.mint()

    // approve the locker
    await this.mockToken.approve(this.locker.address, TOKEN_1_ID)
  })

  describe('init', () => {
    beforeEach(async () => {
      this.locker = await ERC721Locker.new()
    })

    it('Reverts when prover is zero address', async () => {
      await expectRevert(
        this.locker.init(
          Buffer.from('nft.factory.near'),
          ZERO_ADDRESS,
          lockerAdmin
        ),
        "Invalid near prover"
      )
    })

    it('Reverts when token factory is zero bytes', async () => {
      await expectRevert(
        this.locker.init(
          Buffer.from(''),
          nearProver,
          lockerAdmin
        ),
        "Invalid near token factory"
      )
    })
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

  it('unlock from NEAR', async () => {
    await this.locker.lockToken(
      this.mockToken.address,
      TOKEN_1_ID,
      "mynearaccount.near"
    )

    expect(await this.mockToken.ownerOf(TOKEN_1_ID)).to.be.equal(this.locker.address)

    let proof = require('./proof_template.json');
    proof.outcome_proof.outcome.status.SuccessValue = serialize(SCHEMA, 'Unlock', {
      flag: 0,
      tokenId: Buffer.from('00000000000000000000000000000001', 'utf-8'),
      token: hexToBytes(this.mockToken.address),
      recipient: hexToBytes(unlockBeneficiary),
    }).toString('base64');

    await this.locker.unlockToken(borshifyOutcomeProof(proof), 1099);

    expect(await this.mockToken.ownerOf(TOKEN_1_ID)).to.be.equal(unlockBeneficiary)
  });
})
