const hre = require("hardhat");

async function main() {

  const BridgeERC20 = await hre.ethers.getContractFactory("BridgeERC20");
  const bridgeERC20 = await BridgeERC20.deploy("testERC20","T20", 10000000000);

}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});