const { expect } = require('chai');

describe("ERC721MetadataLogger", function () {
  it("Should log erc721 metadata", async function () {
    const SampleERC721 = await ethers.getContractFactory('SampleERC721')
    const sampleERC721 = await SampleERC721.deploy()
    const ERC721MetadataLogger = await ethers.getContractFactory("ERC721MetadataLogger");
    const erc721MetadataLogger = await ERC721MetadataLogger.deploy();
    await erc721MetadataLogger.deployed();

    const tx = await erc721MetadataLogger.log(sampleERC721.address)
    const { events } = await tx.wait()
    const args = events.find(({ event }) => event === 'Log').args
    expect(args.erc721).to.equal(sampleERC721.address)
    expect(args.name).to.equal("SampleERC721")
    expect(args.symbol).to.equal("ERC")
  });
});
