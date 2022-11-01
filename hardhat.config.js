/** @type import('hardhat/config').HardhatUserConfig */
require("@nomiclabs/hardhat-waffle");
require('dotenv').config();

module.exports = {
  solidity: "0.8.17",
  defaultNetwork: "localOKC",
  networks: {
    hardhat: {
    },
    localOKC: {
      url: "http://localhost:8545",
      accounts: [process.env.TEST_USER1_PRIVATE_KEY]
    }
  }
};
