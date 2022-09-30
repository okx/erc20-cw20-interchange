const hre = require("hardhat");

async function main() {

  const BridgeERC20 = await hre.ethers.getContractFactory("BridgeERC20");
  const bridgeERC20 = await BridgeERC20.deploy("testERC20","T20", 10000000000);

  console.log("合约地址：" + bridgeERC20.address)
  console.log("账户余额：" + await bridgeERC20.balanceOf("0x83D83497431C2D3FEab296a9fba4e5FaDD2f7eD0"))
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});