import process from "process";
import hardhat from "hardhat";
import fs from "fs";

async function main() {
    const files = ["ERC721", "ERC721Locker", "Locker"]
    const contracts = ["ERC721BurnableMock", "ERC721Locker", "Locker"]
    
    for (let i = 0; i < files.length; i++) {
        const ERC721Artifact = await hardhat.artifacts.readArtifact(contracts[i])
        fs.writeFileSync("../res/" + files[i] + ".full.abi", JSON.stringify(ERC721Artifact.abi))
        fs.writeFileSync("../res/" + files[i] + ".full.bin", ERC721Artifact.bytecode.replace("0x", ""))
    }
}

main().then().catch(e => {
    console.error(e);
    process.exit(1);
})