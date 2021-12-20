const { expect } = require('chai');
import { SignerWithAddress } from "@nomiclabs/hardhat-ethers/signers";
import hardhat, { ethers } from "hardhat";
const { serialize } = require('rainbow-bridge-lib/rainbow/borsh.js');
const { borshifyOutcomeProof } = require('rainbow-bridge-lib/rainbow/borshify-proof.js');
import {
    ERC721Locker,
    ERC721Locker__factory,
    NearProverMock,
    NearProverMock__factory,
    ERC721BurnableMock,
    ERC721BurnableMock__factory
} from "../typechain";

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

let ERC721LockerContract: ERC721Locker
let NearProverMockContract: NearProverMock
let ERC721BurnableMockContract: ERC721BurnableMock
let accounts: Array<SignerWithAddress>

beforeEach("Setup contract", async () => {
    accounts = await hardhat.ethers.getSigners()

    const ERC721BurnableMockFactory: ERC721BurnableMock__factory = (
        await hardhat.ethers.getContractFactory("ERC721BurnableMock")
    ) as ERC721BurnableMock__factory;

    ERC721BurnableMockContract = (
        await ERC721BurnableMockFactory.deploy()
    ) as ERC721BurnableMock;

    await ERC721BurnableMockContract.deployed();

    const NearProverMockFactory: NearProverMock__factory = (
        await hardhat.ethers.getContractFactory("NearProverMock")
    ) as NearProverMock__factory;

    NearProverMockContract = (
        await NearProverMockFactory.deploy()
    ) as NearProverMock;

    await NearProverMockContract.deployed();

    const ERC721LockerFactory: ERC721Locker__factory = (
        await hardhat.ethers.getContractFactory("ERC721Locker")
    ) as ERC721Locker__factory;

    ERC721LockerContract = (
        await ERC721LockerFactory.deploy(
            Buffer.from('nearnonfuntoken', 'utf-8'),
            NearProverMockContract.address,
            0,
            accounts[0].address,
            1
        )
    ) as ERC721Locker;

    await ERC721LockerContract.deployed();
});

describe("Locking for Near", async () => {
    it("Can lock a token for a given near recipient", async () => {
        await ERC721BurnableMockContract.connect(accounts[0]).mint()

        let tokenId = 1;
        let nearAccount = "mynearaccount.near";

        // account 0 approve the token to the locker
        await ERC721BurnableMockContract.approve(ERC721LockerContract.address, tokenId)

        // call migrateTokenToNear
        expect(await ERC721LockerContract.connect(accounts[0])
            .migrateTokenToNear(
                ERC721BurnableMockContract.address,
                tokenId,
                nearAccount
            )).emit(ERC721LockerContract, "Locked")
            .withArgs(
                ERC721BurnableMockContract.address,
                accounts[0].address,
                String(tokenId),
                nearAccount,
                ""
            )
    });

    it('Can lock multiple tokens for Near', async () => {
        // mint tokens for account 0
        await ERC721BurnableMockContract.connect(accounts[0]).mint()
        await ERC721BurnableMockContract.connect(accounts[0]).mint()
        await ERC721BurnableMockContract.connect(accounts[0]).mint()

        // account 0 approve the token to the locker
        await ERC721BurnableMockContract.approve(ERC721LockerContract.address, 1)
        await ERC721BurnableMockContract.approve(ERC721LockerContract.address, 2)
        await ERC721BurnableMockContract.approve(ERC721LockerContract.address, 3)

        let tokenIds = [1, 2, 3];
        let nearAccount = "mynearaccount.near";

        await ERC721LockerContract.connect(accounts[0])
            .migrateMultipleTokensToNear(
                ERC721BurnableMockContract.address,
                tokenIds,
                nearAccount
            )

    });

    it('unlock from NEAR', async () => {
        let tokenId = 1;
        let nearAccount = "mynearaccount.near";

        await ERC721BurnableMockContract.connect(accounts[0]).mint()
        await ERC721BurnableMockContract.approve(ERC721LockerContract.address, 1)

        await ERC721LockerContract.connect(accounts[0]).migrateTokenToNear(
            ERC721BurnableMockContract.address,
            tokenId,
            nearAccount
        )
        let proof = require('./proof1.json');
        proof.outcome_proof.outcome.status.SuccessValue = serialize(SCHEMA, 'Withdraw', {
            flag: 0,
            token: hardhat.ethers.utils.arrayify(ERC721BurnableMockContract.address),
            recipient: hardhat.ethers.utils.arrayify(accounts[0].address),
            tokenIdStringLength: int32ToBytes(1),
            tokenId: Buffer.from('1', 'utf-8'),
        }).toString('base64')

        await ERC721LockerContract.finishNearToEthMigration(
            borshifyOutcomeProof(proof), 1099
        );
        expect(await ERC721BurnableMockContract.ownerOf(tokenId))
            .equal(accounts[0].address)
    });
});

function int32ToBytes(num: number) {
    let arr = new ArrayBuffer(4); // an Int32 takes 4 bytes
    let view = new DataView(arr);
    view.setUint32(0, num, true); // byteOffset = 0; litteEndian = true as Borsh library is little endian
    return new Uint8Array(arr);
}