async function main() {
  const [deployer] = await ethers.getSigners()
  const deployerAddress = await deployer.getAddress()
  console.log(
    "Deploying mock NFT with the account:",
    deployerAddress
  )

  const ERC721LockerFactory = await ethers.getContractFactory("ERC721BurnableMock")

  const nft = await ERC721LockerFactory.deploy()

  await nft.deployed()

  console.log('nft deployed at', nft.address)

  console.log('Done')
}

main()
  .then(() => process.exit(0))
  .catch(error => {
    console.error(error);
    process.exit(1);
  });
