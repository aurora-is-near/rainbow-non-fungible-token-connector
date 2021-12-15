import hardhat, { ethers } from "hardhat";
import { SignerWithAddress } from "@nomiclabs/hardhat-ethers/signers";
import {
  NFTFactory,
  NFTFactory__factory,
  NearProverMock,
  NearProverMock__factory,
} from "../typechain/";
// import { Signer, Contract, BigNumber } from "ethers";
import chai, { expect } from "chai";
// import { solidity } from "ethereum-waffle";
// import { Artifact } from "hardhat/types";
const {
  borshifyOutcomeProof,
} = require("rainbow-bridge-lib/rainbow/borshify-proof.js");
const { serialize } = require("rainbow-bridge-lib/rainbow/borsh.js");

let NFTFactoryContract: NFTFactory;
let NearMockContract: NearProverMock;
let signer: SignerWithAddress;

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
      0
    )) as NFTFactory;
  });

  it("finaliseNearToEthTransfer", async function () {
    console.log(hardhat.ethers.utils.arrayify(Buffer.from("NFT", "utf-8")));
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
  });

  it("deployBridgedToken", async function () {
    await NFTFactoryContract.deployBridgedToken(
      "SampleNFT"
    );
    await expect(
      NFTFactoryContract.deployBridgedToken(
        "SampleNFT"
      )
    ).to.revertedWith("Contract already exists");
  });

  const proof = require("./proof1.json");
});

function int32ToBytes(num: number) {
  const arr = new ArrayBuffer(4); // an Int32 takes 4 bytes
  const view = new DataView(arr);
  view.setUint32(0, num, true); // byteOffset = 0; litteEndian = true as Borsh library is little endian
  return new Uint8Array(arr);
}
