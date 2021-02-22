async function main() {
  const [deployer] = await ethers.getSigners()
  const deployerAddress = await deployer.getAddress()
  console.log(
    "Deploying NFT locker with the account:",
    deployerAddress
  )

  const ERC721LockerFactory = await ethers.getContractFactory("ERC721Locker")

  const locker = await ERC721LockerFactory.deploy()

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
