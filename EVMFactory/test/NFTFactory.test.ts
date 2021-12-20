import { expect } from "chai";
import hardhat, { ethers } from "hardhat";
import { SignerWithAddress } from "@nomiclabs/hardhat-ethers/signers";
import {
  NFTFactory,
  NFTFactory__factory,
  NearProverMock,
  NearProverMock__factory,
  BridgedNFT,
} from "../typechain/";

const {
  borshifyOutcomeProof,
} = require("rainbow-bridge-lib/rainbow/borshify-proof.js");
const { serialize } = require("rainbow-bridge-lib/rainbow/borsh.js");

let NFTFactoryContract: NFTFactory;
let NearMockContract: NearProverMock;
let signer: SignerWithAddress;
const proof = require("./proof1.json");

const SCHEMA = {
  Locked: {
    kind: "struct",
    fields: [
      ["flag", "u8"],
      ["recipient", [20]],
      ["tokenAccountIdStringLength", [4]],
      ["tokenAccountId", [3]],
      ["tokenIdStringLength", [4]],
      ["tokenId", [2]],
    ],
  },
};

describe("NodeOperator", function () {
  beforeEach(async function () {
    const accounts = await ethers.getSigners();
    signer = accounts[0];

    const nearMockFactory: NearProverMock__factory =
      (await hardhat.ethers.getContractFactory(
        "NearProverMock"
      )) as NearProverMock__factory;

    NearMockContract = (await nearMockFactory.deploy()) as NearProverMock;

    const nftFactoryArtifact: NFTFactory__factory =
      (await hardhat.ethers.getContractFactory(
        "NFTFactory"
      )) as NFTFactory__factory;
    NFTFactoryContract = (await nftFactoryArtifact.deploy(
      NearMockContract.address,
      Buffer.from("nearnonfuntoken", "utf-8"),
      0,
      signer.address,
      0
    )) as NFTFactory;
  });

  it("Success finaliseNearToEthTransfer", async function () {
    await NFTFactoryContract.deployBridgedToken("NFT", "", "");
    const bridgedNFTArtifact = await hardhat.artifacts.readArtifact(
      "BridgedNFT"
    );
    const bridgedNFTAddress = await NFTFactoryContract.bridgedNFTs("NFT");
    const bridgedNFT: BridgedNFT = (await ethers.getContractAt(
      bridgedNFTArtifact.abi,
      bridgedNFTAddress
    )) as BridgedNFT;

    proof.outcome_proof.outcome.status.SuccessValue = serialize(
      SCHEMA,
      "Locked",
      {
        flag: 0,
        recipient: hardhat.ethers.utils.arrayify(signer.address),
        tokenAccountIdStringLength: int32ToBytes(3),
        tokenAccountId: Buffer.from("NFT", "utf-8"),
        tokenIdStringLength: int32ToBytes(2),
        tokenId: Buffer.from("22", "utf-8"),
      }
    ).toString("base64");

    await NFTFactoryContract.finaliseNearToEthTransfer(
      borshifyOutcomeProof(proof),
      10
    );
    expect(await bridgedNFT.balanceOf(signer.address)).equal(1);
    expect(await bridgedNFT.ownerOf(22)).equal(signer.address);
  });

  it("Fail finaliseNearToEthTransfer Contract not deployed", async function () {
    proof.outcome_proof.outcome.status.SuccessValue = serialize(
      SCHEMA,
      "Locked",
      {
        flag: 0,
        recipient: hardhat.ethers.utils.arrayify(signer.address),
        tokenAccountIdStringLength: int32ToBytes(3),
        tokenAccountId: Buffer.from("NFT", "utf-8"),
        tokenIdStringLength: int32ToBytes(2),
        tokenId: Buffer.from("22", "utf-8"),
      }
    ).toString("base64");

    await expect(
      NFTFactoryContract.finaliseNearToEthTransfer(
        borshifyOutcomeProof(proof),
        10
      )
    ).revertedWith("Contract not deployed");
  });

  it("deployBridgedToken", async function () {
    await NFTFactoryContract.deployBridgedToken("SampleNFT", "", "");
    await expect(
      NFTFactoryContract.deployBridgedToken("SampleNFT", "", "")
    ).to.revertedWith("Contract already deployed");
  });

  it("Success withdraw", async function () {
    await NFTFactoryContract.deployBridgedToken("NFT", "", "");
    const bridgedNFTArtifact = await hardhat.artifacts.readArtifact(
      "BridgedNFT"
    );
    const bridgedNFTAddress = await NFTFactoryContract.bridgedNFTs("NFT");
    const bridgedNFT: BridgedNFT = (await ethers.getContractAt(
      bridgedNFTArtifact.abi,
      bridgedNFTAddress
    )) as BridgedNFT;
    await mintNFT("22", "NFT");
    expect(await bridgedNFT.withdrawNFT(22, "reciver"))
      .emit(bridgedNFT, "Withdraw")
      .withArgs(bridgedNFT.address, signer.address, "NFT", 22, "reciver");
  });

  it("Fail withdraw, contract is paused", async function () {
    await NFTFactoryContract.deployBridgedToken("NFT", "", "");
    const bridgedNFTArtifact = await hardhat.artifacts.readArtifact(
      "BridgedNFT"
    );
    const bridgedNFTAddress = await NFTFactoryContract.bridgedNFTs("NFT");
    const bridgedNFT: BridgedNFT = (await ethers.getContractAt(
      bridgedNFTArtifact.abi,
      bridgedNFTAddress
    )) as BridgedNFT;
    await mintNFT("22", "NFT");
    await NFTFactoryContract.setPauseBridgedWithdraw(true);
    await expect(bridgedNFT.withdrawNFT(22, "reciver")).revertedWith(
      "Withdrawal is disabled"
    );
  });

  it("Fail proof used many times", async function () {
    await NFTFactoryContract.deployBridgedToken("NFT", "", "");
    await mintNFT("22", "NFT");
    await expect(mintNFT("22", "NFT")).revertedWith(
      "The lock event cannot be reused"
    );
  });
});

// accountId == NFT
async function mintNFT(tokenId: String, accountId: String) {
  proof.outcome_proof.outcome.status.SuccessValue = serialize(
    SCHEMA,
    "Locked",
    {
      flag: 0,
      recipient: hardhat.ethers.utils.arrayify(signer.address),
      tokenAccountIdStringLength: int32ToBytes(3),
      tokenAccountId: Buffer.from(accountId, "utf-8"),
      tokenIdStringLength: int32ToBytes(2),
      tokenId: Buffer.from(tokenId, "utf-8"),
    }
  ).toString("base64");

  await NFTFactoryContract.finaliseNearToEthTransfer(
    borshifyOutcomeProof(proof),
    10
  );
}

function int32ToBytes(num: number) {
  const arr = new ArrayBuffer(4); // an Int32 takes 4 bytes
  const view = new DataView(arr);
  view.setUint32(0, num, true); // byteOffset = 0; litteEndian = true as Borsh library is little endian
  return new Uint8Array(arr);
}
