var wasm = require("cosmwasm");
var amino = require("@cosmjs/amino");
var encoding =require("@cosmjs/encoding");
var fs = require("fs");

// This is your rpc endpoint
const rpcEndpoint = "localhost:26657";

async function deployWASMContract(evmContract) {
    
    let signer = await amino.Secp256k1Wallet.fromKey(encoding.fromHex(process.env.TEST_USER1_PRIVATE_KEY), "ex");
    let [alice,] = await signer.getAccounts();

    const cwclient = await wasm.SigningCosmWasmClient.connectWithSigner(rpcEndpoint,signer);

    var balance = await cwclient.getBalance(alice.address,"okt")

    let filedata =fs.readFileSync('./WASMContracts/target/wasm32-unknown-unknown/release/cw_erc20.wasm')

    //update the contract
    var result = await cwclient.upload(alice.address,filedata,{"amount":wasm.parseCoins("200000000000000000wei"),"gas":"20000000"})

    var initMsg = {"name":"testERC","symbol":"TRC", "decimals": 18,"evm_contract" :evmContract};

    //init the contract
    var res2 = await cwclient.instantiate(alice.address, result.codeId, initMsg, "hello world", {"amount":wasm.parseCoins("200000000000000000wei"),"gas":"20000000"},{"funds":[{"denom":"okt","amount":"100000000000000000000"}],"admin":alice.address})

    return res2;
}

module.exports = {
    deployWASMContract,
}