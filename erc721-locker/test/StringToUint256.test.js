const { BN, constants, expectEvent, expectRevert } = require('@openzeppelin/test-helpers');
const { ZERO_ADDRESS } = constants;

const { expect } = require('chai');

const StringToUint256Tester = artifacts.require('StringToUint256Tester')

contract('StringToUint256Tester', function ([deployer, ...otherAccounts]) {
  beforeEach(async () => {
    this.tester = await StringToUint256Tester.new()
  })

  it('Correctly converts a string to an bignumber', async () => {
    const toConvert = '562700';

    const result = await this.tester.convert(toConvert)

    expect(result).to.be.bignumber.equal(toConvert)
  })
})
