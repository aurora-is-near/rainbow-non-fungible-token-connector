# ERC721 Locker

The entry point for locking up tokens that get migrated to Near

![](https://i.imgur.com/DavgJDz.png)

If Alice wants to migrate NFTs to Near, she locks her NFTs in the ERC721 locker via the locker's proxy contract (proxy allows the locker to be later upgraded)

Upgrading the locker implementation can be done by an EOA with the admin role in the Near access controls contract

### Installing Dependencies

```
yarn
```

### Running tests

```
yarn test
```

### Running coverage

```
yarn coverage
```

### Setting up ERC721 locker with a proxy

#### See `scripts/` for deploying to your chosen network

### Step 1

Deploy `ERC721Locker.sol`

*Make a note of the address*

### Step 2

Deploy `NearAccessControls.sol` if it has not been deployed

By default, the sender is the first account given the `admin` role. That account can then grant admin and other roles to other EOAs or smart contracts

*Make a note of the address*

### Step 3

Deploy `NearUpgradeableProxy` with the following params:

- Locker address from `Step 1`
- Near access controls from `Step 2`
- Encoded call to the locker's `init` method 

Encoding the call to the locker's init method means we can deploy the proxy and initialize the implementation in one transaction.

Here is a web 3 example of deploying a proxy:

```
this.proxy = await NearUpgradeableProxy.new(
      this.lockerLogic.address,
      this.accessControls.address,
      await new web3.eth.Contract(ERC721LockerABI).methods.init(
        Buffer.from('nft.factory.near'),
        nearProver
      ).encodeABI(),
      {from: deployer}
    )
```

In this case, we are supplying the account of the near NFT factory and near prover to the init method 
