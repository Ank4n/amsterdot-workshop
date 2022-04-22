pragma solidity ^0.8.9;

contract Simple {
    address _owner;
    uint256 _play;

    constructor(uint256 p) {
        _owner = msg.sender;
        _play = p;
    }

    function owner() public view returns (address) {
        return _owner;
    }

    function play(uint256 round, uint256 prevPlay, uint256 otherPrevPlay) public view returns (uint256) {
        return _play;
    }
}