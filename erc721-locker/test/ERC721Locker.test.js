const { BN, constants, expectEvent, expectRevert } = require('@openzeppelin/test-helpers');
const { ZERO_ADDRESS } = constants;

const { expect } = require('chai');

const { serialize } = require('rainbow-bridge-lib/rainbow/borsh.js');
const { borshifyOutcomeProof } = require('rainbow-bridge-lib/rainbow/borshify-proof.js');

const { toWei, fromWei, hexToBytes } = web3.utils;

const ERC721Locker = artifacts.require('ERC721Locker')
const NearProverMock = artifacts.require('test/NearProverMock')
const ERC721BurnableMock = artifacts.require('ERC721BurnableMock')

const SCHEMA = {
  'Withdraw': {
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

    this.locker = await ERC721Locker.new(
      Buffer.from('nearnonfuntoken', 'utf-8'),
      this.prover.address,
      0,
      lockerAdmin,
      1
    )

    // deploy a mock token and mint the first NFT
    this.mockToken = await ERC721BurnableMock.new()
    await this.mockToken.mint()

    // approve the locker
    await this.mockToken.approve(this.locker.address, TOKEN_1_ID)
  })

  describe.only('init', () => {
    it('Reverts when prover is zero address', async () => {
      await expectRevert(
        ERC721Locker.new(
          Buffer.from('nft.factory.near'),
          ZERO_ADDRESS,
          0,
          lockerAdmin,
          1
        ),
        "Invalid near prover"
      )
    })

    it('Reverts when token factory is zero bytes', async () => {
      await expectRevert(
        ERC721Locker.new(
          Buffer.from(''),
          nearProver,
          0,
          lockerAdmin,
          1
        ),
        "Invalid near token factory"
      )
    })
  })

  describe.only('Locking for Near native', () => {
    it('Can lock a token for a given near recipient', async () => {
      const { receipt } = await this.locker.lockToken(
        this.mockToken.address,
        TOKEN_1_ID,
        "mynearaccount.near"
      )

      await expectEvent(receipt, 'Locked', {
        token: this.mockToken.address,
        sender: deployer,
        tokenId: TOKEN_1_ID,
        accountId: "mynearaccount.near"
      })

      expect(await this.mockToken.ownerOf(TOKEN_1_ID)).to.be.equal(this.locker.address)
    })
  })

  it('unlock from NEAR', async () => {
    await this.locker.lockToken(
      this.mockToken.address,
      TOKEN_1_ID,
      "mynearaccount.near"
    )

    expect(await this.mockToken.ownerOf(TOKEN_1_ID)).to.be.equal(this.locker.address)

    // todo how to serialise bytes for token
    let proof = require('./proof_template.json');
    proof.outcome_proof.outcome.status.SuccessValue = serialize(SCHEMA, 'Withdraw', {
      flag: 0,
      tokenId: Buffer.from('00000000000000000000000000000001', 'utf-8'),
      token: hexToBytes(this.mockToken.address),
      recipient: hexToBytes(unlockBeneficiary),
    }).toString('base64');

    await this.locker.unlockToken(borshifyOutcomeProof(proof), 1099);

    expect(await this.mockToken.ownerOf(TOKEN_1_ID)).to.be.equal(unlockBeneficiary)
  });
})
