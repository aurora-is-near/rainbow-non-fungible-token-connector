const prompt = require('prompt-sync')();
const ERC721LockerABI = require('../artifacts/contracts/ERC721Locker.sol/ERC721Locker.json').abi;
const Web3 = require('web3');

async function main() {
  const [deployer] = await ethers.getSigners()
  const deployerAddress = await deployer.getAddress()
  console.log(
    "Deploying proxy to NFT locker with the account:",
    deployerAddress
  )

  const lockerAddress = prompt('ERC721Locker address? ');
  const accessControlsAddress = prompt('NearAccessControls address? ');
  const nearFactoryAccount = prompt('Near NFT factory account ID? ');
  const proverAddress = prompt('Near prover address? ');

  console.log(`\nERC721Locker address: ${lockerAddress}`)
  console.log(`\nNearAccessControls address: ${accessControlsAddress}`)
  console.log(`\nFactory Account ID: ${nearFactoryAccount}`)
  console.log(`\nNear prover address: ${proverAddress}`)

  prompt('If happy, hit enter...');

  const NearUpgradeableProxyFactory = await ethers.getContractFactory("NearUpgradeableProxy")

  const proxy = await NearUpgradeableProxyFactory.deploy(
    lockerAddress,
    accessControlsAddress,
    await new Web3.eth.Contract(ERC721LockerABI).methods.init(
      Buffer.from(nearFactoryAccount),
      proverAddress
    ).encodeABI(),
  )

  await proxy.deployed()

  console.log('Proxy deployed at', proxy.address)

  console.log('Done')
}

main()
  .then(() => process.exit(0))
  .catch(error => {
    console.error(error);
    process.exit(1);
  });
