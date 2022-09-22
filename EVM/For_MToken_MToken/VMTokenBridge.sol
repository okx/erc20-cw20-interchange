// SPDX-License-Identifier: MIT

pragma solidity 0.8.7;

import "./ERC20.sol";

contract VMTokenBridge {

    address internal constant moduleAddress = 0xc63cf6c8E1f3DF41085E9d8Af49584dae1432b4f;
    string internal immutable wasmContractAddress;


    event Initialize(string wasmContractAddress);
    event __OKCSendToWasm(string wasmAddr, string recipient, uint256 amount);

    modifier onlyWasm() {
        require(msg.sender == moduleAddress, "Only Wasm specified address can call");
    }

    modifier onlyWasmContract(string calldata caller){
        require(caller == wasmContractAddress, "Only specified wasm contract can call");
    }

    function initialize(string calldata _wasmContractAddress) public {

        wasmContractAddress = _wasmContractAddress;

        emit Initialize(wasmContractAddress);
    }

    function mintERC20(
        string calldata caller, 
        address recipient,
        uint256 amount
    ) 
    external override 
    onlyWasm 
    onlyWasmContract(caller) 
    returns (bool) 
    {
        _mint(recipient, amount);
        return true;
    }

    function send_to_wasm(string memory recipient,string memory wasmContract , uint256 amount) public {
        _burn(msg.sender, amount);
        emit __OKCSendToWasm(wasmContract,recipient, amount);
    }
}