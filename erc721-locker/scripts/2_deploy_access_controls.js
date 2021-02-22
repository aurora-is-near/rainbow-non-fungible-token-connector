async function main() {
  const [deployer] = await ethers.getSigners()
  const deployerAddress = await deployer.getAddress()
  console.log(
    "Deploying access controls with the account:",
    deployerAddress
  )

  const NearAccessControlsFactory = await ethers.getContractFactory("NearAccessControls")

  const accessControls = await NearAccessControlsFactory.deploy()

  await accessControls.deployed()

  console.log('Access controls deployed at', accessControls.address)

  console.log('Done')
}

main()
  .then(() => process.exit(0))
  .catch(error => {
    console.error(error);
    process.exit(1);
  });
