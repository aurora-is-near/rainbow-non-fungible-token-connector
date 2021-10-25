const { BN, constants, expectEvent, expectRevert } = require('@openzeppelin/test-helpers');
const { ZERO_ADDRESS } = constants;

const { expect } = require('chai');

const { serialize } = require('rainbow-bridge-lib/rainbow/borsh.js');
const { borshifyOutcomeProof } = require('rainbow-bridge-lib/rainbow/borshify-proof.js');

const { hexToBytes } = web3.utils;

function int32ToBytes (num) {
  let arr = new ArrayBuffer(4); // an Int32 takes 4 bytes
  let view = new DataView(arr);
  view.setUint32(0, num, true); // byteOffset = 0; litteEndian = true as Borsh library is little endian
  return new Uint8Array(arr);
}

const ERC721Locker = artifacts.require('ERC721Locker')
const NearProverMock = artifacts.require('test/NearProverMock')
const ERC721BurnableMock = artifacts.require('ERC721BurnableMock')

const SCHEMA = {
  'Withdraw': {
    kind: 'struct', fields: [
      ['flag', 'u8'],
      ['token', [20]],
      ['recipient', [20]],
      ['tokenIdStringLength', [4]],
      ['tokenId', [1]]
    ]
  }
};

contract('ERC721Locker', function ([deployer, nearProver, nearEvmBeneficiary, unlockBeneficiary, lockerAdmin, ...otherAccounts]) {
  const TOKEN_1_ID = new BN('1')
  const TOKEN_2_ID = new BN('2')
  const TOKEN_3_ID = new BN('3')

  beforeEach(async () => {
    this.prover = await NearProverMock.new();

    this.locker = await ERC721Locker.new(
      Buffer.from('nearnonfuntoken', 'utf-8'),
      this.prover.address,
      0,
      lockerAdmin,
      0
    )

    // deploy a mock token and mint the first NFT
    this.mockToken = await ERC721BurnableMock.new()
    await this.mockToken.mint()
    await this.mockToken.mint()
    await this.mockToken.mint()

    // approve the locker
    await this.mockToken.setApprovalForAll(this.locker.address, true)
  })

  describe('init', () => {
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

  describe('Locking for Near', () => {
    it('Can lock a token for a given near recipient', async () => {
      const { receipt } = await this.locker.migrateTokenToNear(
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

    it('Can lock multiple tokens for Near', async () => {
      await this.locker.migrateMultipleTokensToNear(
        this.mockToken.address,
        [TOKEN_1_ID, TOKEN_2_ID, TOKEN_3_ID],
        "mynearaccount.near"
      )

      expect(await this.mockToken.ownerOf(TOKEN_1_ID)).to.be.equal(this.locker.address)
      expect(await this.mockToken.ownerOf(TOKEN_2_ID)).to.be.equal(this.locker.address)
      expect(await this.mockToken.ownerOf(TOKEN_3_ID)).to.be.equal(this.locker.address)
    })
  })

  it('unlock from NEAR', async () => {
    // lock token and assume we have migrated to Near
    await this.locker.migrateTokenToNear(
      this.mockToken.address,
      TOKEN_1_ID,
      "mynearaccount.near"
    )

    expect(await this.mockToken.ownerOf(TOKEN_1_ID)).to.be.equal(this.locker.address)

    let proof = require('./proof_template.json');
    proof.outcome_proof.outcome.status.SuccessValue = serialize(SCHEMA, 'Withdraw', {
      flag: 0,
      token: hexToBytes(this.mockToken.address),
      recipient: hexToBytes(unlockBeneficiary),
      tokenIdStringLength: int32ToBytes(1),
      tokenId: Buffer.from('1', 'utf-8'),
    }).toString('base64')

    await this.locker.finishNearToEthMigration(borshifyOutcomeProof(proof), 1099);

    expect(await this.mockToken.ownerOf(TOKEN_1_ID)).to.be.equal(unlockBeneficiary)
  });
})
