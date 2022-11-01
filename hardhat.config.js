/** @type import('hardhat/config').HardhatUserConfig */
require("@nomiclabs/hardhat-waffle");

module.exports = {
  solidity: "0.8.17",
  networks: {
    hardhat: {
    },
    localOKC: {
      url: "http://localhost:8545",
      accounts: [""]
    }
  }
};
