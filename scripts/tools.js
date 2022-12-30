var encoding =require("@cosmjs/encoding");


function evmAddress2ExAddress(addr){
    return encoding.toBech32("ex", encoding.fromHex(addr.substring(2)))
}

function exAddress2evmAddress(addr){
    return ("0x"+encoding.toHex(encoding.fromBech32(addr).data))
}

module.exports = {
    evmAddress2ExAddress,
    exAddress2evmAddress
}
    