const { BN, constants, expectEvent, expectRevert } = require('@openzeppelin/test-helpers');
const { ZERO_ADDRESS } = constants;

const { expect } = require('chai');

const StringToUint256Tester = artifacts.require('StringToUint256Tester')

contract('StringToUint256Tester', function ([deployer, ...otherAccounts]) {
  beforeEach(async () => {
    this.tester = await StringToUint256Tester.new()
  })

  it('Correctly converts a string to an bignumber', async () => {
    const toConvert = '56270099';

    const result = await this.tester.convert(toConvert)

    expect(result).to.be.bignumber.equal(toConvert)
  })

  it('Correctly converts max uint', async () => {
    const toConvert = '115792089237316195423570985008687907853269984665640564039457584007913129639935';

    const result = await this.tester.convert(toConvert)

    expect(result).to.be.bignumber.equal(toConvert)
  })

  it('Reverts when string causes an overflow', async () => {
    const toConvert = '115792089237316195423570985008687907853269984665640564039457584007913129639936';

    await expectRevert(
      this.tester.convert(toConvert),
      "SafeMath: addition overflow"
    )
  })

  it('Reverts if string is not a number', async () => {
    await expectRevert(
      this.tester.convert('59x'),
      "String is not a number"
    )
  })

  it('Reverts if string is too long', async () => {
    await expectRevert(
      this.tester.convert('11579208923731619542357098500868790785326998466564056403945758400791312963993695'),
      "Number in string could cause overflow due to length"
    )
  })
})
