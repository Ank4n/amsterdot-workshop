pragma solidity ^0.8.9;

contract Advanced {
    address _owner;
    uint256 _prevPrevPlay;

    constructor() {
        _owner = msg.sender;
    }

    function owner() public view returns (address) {
        return _owner;
    }

    function play(uint256 round, uint256 prevPlay, uint256 otherPrevPlay) public returns (uint256) {
        if (round == 0) {
            return 302;
        }
        if (otherPrevPlay == 302) {
            return 302;
        }
        if (otherPrevPlay == round - 1) {
            return round + 2;
        }
        if (otherPrevPlay == 0) {
            return 302;
        }
        if (otherPrevPlay == (_prevPrevPlay + 2) % 3) {
            return prevPlay + 1;
        }
        _prevPrevPlay = prevPlay;
        return uint256(blockhash(block.number)) + round;
    }
}