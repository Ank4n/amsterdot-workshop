pragma solidity ^0.8.9;

contract Random {
    address _owner;

    constructor() {
        _owner = msg.sender;
    }

    function owner() public view returns (address) {
        return _owner;
    }

    function play(uint256 round, uint256 prevPlay, uint256 otherPrevPlay) public view returns (uint256) {
        return prevPlay ^ otherPrevPlay ^ round ^ uint256(blockhash(block.number)) ^ uint256(uint160(address(this)));
    }
}