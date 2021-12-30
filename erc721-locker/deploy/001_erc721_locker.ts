import process from "process";
import fs from "fs";
import hardhat, { upgrades } from "hardhat";
import {
    ERC721Locker,
    ERC721Locker__factory
} from "../typechain";

async function main() {
    let nearProofAddress
    if (hardhat.network.name !== "coverage") {
        const config = JSON.parse(fs.readFileSync("config.json").toString())
        nearProofAddress = config[hardhat.network.name].nearProver
    }

    const ERC721LockerFactory: ERC721Locker__factory = (
        await hardhat.ethers.getContractFactory("ERC721Locker")
    ) as ERC721Locker__factory;

    const ERC721LockerContract = (
        await upgrades.deployProxy(ERC721LockerFactory, [
            Buffer.from('nearnonfuntoken', 'utf-8'),
            nearProofAddress,
            0,
            1
        ])
    ) as ERC721Locker;

    await ERC721LockerContract.deployed();
}

main()
    .then(() => process.exit(0))
    .catch((error) => {
       
        console.error(error);
        process.exit(1);
    });