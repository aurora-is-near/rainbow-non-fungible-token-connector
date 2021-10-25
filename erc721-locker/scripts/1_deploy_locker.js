async function main() {
  const [deployer] = await ethers.getSigners()
  const deployerAddress = await deployer.getAddress()
  console.log(
    "Deploying NFT locker with the account:",
    deployerAddress
  )

  const ERC721LockerFactory = await ethers.getContractFactory("ERC721Locker")

  const locker = await ERC721LockerFactory.deploy(
    Buffer.from("nft-factory.testnet", 'utf-8'),
    '0xb3df48b0ea3e91b43226fb3c5eb335b7e3d76faa',
    0,
    deployerAddress,
    0
  )

  await locker.deployed()

  console.log('Locker deployed at', locker.address)

  console.log('Done')
}

main()
  .then(() => process.exit(0))
  .catch(error => {
    console.error(error);
    process.exit(1);
  });
