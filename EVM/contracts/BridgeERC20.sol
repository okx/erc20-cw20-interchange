// SPDX-License-Identifier: MIT
pragma solidity ^0.8.7;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";

contract BridgeERC20 is ERC20, Initializable{

    address public constant MODULE_ADDRESS = 0xc63cf6c8E1f3DF41085E9d8Af49584dae1432b4f;
    string internal wasmContractAddress;


    event Initialize(string wasmContractAddress);
    event __OKCSendToWasm(string wasmAddr, string recipient, uint256 amount);

    modifier onlyWasm() {
        require(msg.sender == MODULE_ADDRESS, "Only Wasm specified address can call");
        _;
    }

    modifier onlyWasmContract(string calldata caller){
        require(keccak256(abi.encodePacked(caller))  == keccak256(abi.encodePacked(wasmContractAddress)) , "Only specified wasm contract can call");
        _;
    }

    constructor(string memory _name, string memory _symbol, uint256 _totalSupply) ERC20(_name, _symbol){
        _mint(msg.sender, _totalSupply);
    }

    function initialize(string calldata _wasmContractAddress) public initializer{

        wasmContractAddress = _wasmContractAddress;

        emit Initialize(wasmContractAddress);
    }

    function mintERC20(
        string calldata caller, 
        address recipient,
        uint256 amount
    ) 
    external 
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