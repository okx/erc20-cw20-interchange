/** @type import('hardhat/config').HardhatUserConfig */
require("@nomiclabs/hardhat-waffle");

module.exports = {
  solidity: "0.8.17",
  networks: {
    hardhat: {
    },
    localOKC: {
      url: "http://localhost:8545",
      accounts: ["171786c73f805d257ceb07206d851eea30b3b41a2170ae55e1225e0ad516ef42"]
    }
  }
};
