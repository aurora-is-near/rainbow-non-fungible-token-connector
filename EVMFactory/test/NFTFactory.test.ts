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
      ["tokenUriStringLength", [4]],
      ["tokenUri", [8]],
    ],
  },
};

const SCHEMA_METADATA = {
  Log: {
    kind: "struct",
    fields: [
      ["flag", "u8"],
      ["accountIdStringLength", [4]],
      ["accountId", [3]],
      ["nameStringLength", [4]],
      ["name", [4]],
      ["symbolStringLength", [4]],
      ["symbol", [4]],
      ["iconStringLength", [4]],
      ["icon", [4]],
      ["baseUriStringLength", [4]],
      ["baseUri", [4]],
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

    await NFTFactoryContract.finaliseNearToEthTransfer(
      borshifyOutcomeProof(setupProof("22", "NFT", "tokenuri")),
      10
    );
    expect(await bridgedNFT.balanceOf(signer.address)).equal(1);
    expect(await bridgedNFT.ownerOf(22)).equal(signer.address);
  });

  it("Success updateMetadata", async function () {
    await NFTFactoryContract.deployBridgedToken("NFT", "", "");
    const bridgedNFTArtifact = await hardhat.artifacts.readArtifact(
      "BridgedNFT"
    );
    const bridgedNFTAddress = await NFTFactoryContract.bridgedNFTs("NFT");
    const bridgedNFT: BridgedNFT = (await ethers.getContractAt(
      bridgedNFTArtifact.abi,
      bridgedNFTAddress
    )) as BridgedNFT;
    const proof = setupProofForMetadata("NFT", "NAME", "SYMB");
    expect(
      await NFTFactoryContract.update_metadata(borshifyOutcomeProof(proof), 10)
    );

    expect(await bridgedNFT.name()).eq("NAME");
    expect(await bridgedNFT.symbol()).eq("SYMB");
  });

  it("Fail finaliseNearToEthTransfer Contract not deployed", async function () {
    await expect(
      NFTFactoryContract.finaliseNearToEthTransfer(
        borshifyOutcomeProof(setupProof("22", "NFT", "tokenuri")),
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
    await mintNFT("22", "NFT", "tokenuri");
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
    await mintNFT("22", "NFT", "tokenuri");
    await NFTFactoryContract.setPauseBridgedWithdraw(true);
    await expect(bridgedNFT.withdrawNFT(22, "reciver")).revertedWith(
      "Withdrawal is disabled"
    );
  });

  it("Fail proof used many times", async function () {
    await NFTFactoryContract.deployBridgedToken("NFT", "", "");
    await mintNFT("22", "NFT", "tokenuri");
    await expect(mintNFT("22", "NFT", "tokenuri")).revertedWith(
      "The lock event cannot be reused"
    );
  });

  it("Success get token uri", async function () {
    await NFTFactoryContract.deployBridgedToken("NFT", "", "");
    const bridgedNFTArtifact = await hardhat.artifacts.readArtifact(
      "BridgedNFT"
    );
    const bridgedNFTAddress = await NFTFactoryContract.bridgedNFTs("NFT");
    const bridgedNFT: BridgedNFT = (await ethers.getContractAt(
      bridgedNFTArtifact.abi,
      bridgedNFTAddress
    )) as BridgedNFT;
    await mintNFT("22", "NFT", "tokenuri");
    expect(await bridgedNFT.tokenURI(22)).eq("tokenuri");
  });
});

// accountId == NFT
async function mintNFT(tokenId: String, accountId: String, tokenuri: String) {
  await NFTFactoryContract.finaliseNearToEthTransfer(
    borshifyOutcomeProof(setupProof(tokenId, accountId, tokenuri)),
    10
  );
}

function int32ToBytes(num: number) {
  const arr = new ArrayBuffer(4); // an Int32 takes 4 bytes
  const view = new DataView(arr);
  view.setUint32(0, num, true); // byteOffset = 0; litteEndian = true as Borsh library is little endian
  return new Uint8Array(arr);
}

function setupProof(tokenId: String, accountId: String, tokenUri: String) {
  const proof = require("./proof1.json");
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
      tokenUriStringLength: int32ToBytes(8),
      tokenUri: Buffer.from(tokenUri, "utf-8"),
    }
  ).toString("base64");

  return proof;
}

function setupProofForMetadata(
  accountId: String,
  name: String,
  symbol: String
) {
  const proof = require("./proof1.json");
  proof.outcome_proof.outcome.status.SuccessValue = serialize(
    SCHEMA_METADATA,
    "Log",
    {
      flag: 0,
      accountIdStringLength: int32ToBytes(3),
      accountId: Buffer.from(accountId, "utf-8"),
      nameStringLength: int32ToBytes(4),
      name: Buffer.from(name, "utf-8"),
      symbolStringLength: int32ToBytes(4),
      symbol: Buffer.from(symbol, "utf-8"),
      iconStringLength: int32ToBytes(4),
      icon: Buffer.from("icon", "utf-8"),
      baseUriStringLength: int32ToBytes(4),
      baseUri: Buffer.from("buri", "utf-8"),
    }
  ).toString("base64");

  return proof;
}
