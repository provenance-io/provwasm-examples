# Hash Order Book Matching Example

This is an example order book that explores matching bids and asks on-chain.

For simplicity, ask orders must be `nhash` in increments of 1,000,000,000. Bid order denom can be
specified during instantiation, but it must be backed by an unrestricted marker.

The matching algorithm used in this example is __Price-Time-Priority/FIFO__

NOTE: This is not truly a decentralized orderbook on blockchain - it requires a privileged account
to run the matching algorithm (and pay fees) no more than once per block. This admin account is
prohibited from placing bid/ask orders. However, using an admin account does not prevent orderbook
manipulation - the organization controlling the match executions could have a separate address used
for placing bid/ask orders. In theory, they could "watch" the orderbook, then execute the match
when their bid/ask account has orders in an optimal position. In order for this to be truly
decentralized, there needs to be a begin/end block hook (or something similar) that executes
matching - which is not currently available to CosmWasm contracts.

## TODO

- Cancel bid
- Cancel ask
- Add 24hr order expiration
- Purge expired orders

## Blockchain Setup

Clear all current state, install the `provenanced` command, then start a 4-node localnet.

```bash
make clean
make install
make localnet-start
```

## Accounts

Accounts needs to be set up for bidders and askers

Buyer 1

```bash
provenanced keys add buyer1 \
    --home build/node0 --keyring-backend test --testnet --hd-path "44'/1'/0'/0/0" --output json | jq
```

Buyer 1

```bash
provenanced keys add buyer2 \
    --home build/node0 --keyring-backend test --testnet --hd-path "44'/1'/0'/0/0" --output json | jq
```

Seller 1

```bash
provenanced keys add seller1 \
    --home build/node0 --keyring-backend test --testnet --hd-path "44'/1'/0'/0/0" --output json | jq
```

Seller 2

```bash
provenanced keys add seller2 \
    --home build/node0 --keyring-backend test --testnet --hd-path "44'/1'/0'/0/0" --output json | jq
```

## Funding Sellers

Fund the seller accounts with `nhash`

```bash
provenanced tx bank send \
    $(provenanced keys show -a node0 --home build/node0 --keyring-backend test --testnet) \
    $(provenanced keys show -a seller1 --home build/node0 --keyring-backend test --testnet) \
    100000000000nhash \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 2000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

```bash
provenanced tx bank send \
    $(provenanced keys show -a node0 --home build/node0 --keyring-backend test --testnet) \
    $(provenanced keys show -a seller2 --home build/node0 --keyring-backend test --testnet) \
    100000000000nhash \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 2000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

### Stablecoin Marker

A marker must be created in order to have a supply of stablecoin required for exchanging nhash.

```bash
provenanced tx marker new 1000stablecoin5201 \
    --type COIN \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 2000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Grant access on the marker to the `node0` account.

```bash
provenanced tx marker grant \
    $(provenanced keys show -a node0 --home build/node0 --keyring-backend test --testnet) \
    stablecoin5201 \
    admin,burn,deposit,delete,mint,withdraw \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 2000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Finalize the marker

```bash
provenanced tx marker finalize stablecoin5201 \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 2000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Activate the marker, minting and escrowing the supply

```bash
provenanced tx marker activate stablecoin5201 \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 2000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

## Funding Buyers

Fund the buyer accounts from the `stablecoin` marker.

```bash
provenanced tx marker withdraw stablecoin5201 \
    100stablecoin5201 \
    $(provenanced keys show -a buyer1 --home build/node0 --keyring-backend test --testnet) \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 20000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

```bash
provenanced tx marker withdraw stablecoin5201 \
    100stablecoin5201 \
    $(provenanced keys show -a buyer2 --home build/node0 --keyring-backend test --testnet) \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 5000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Fund the buyer accounts with a small amount of `nhash` to pay network fees.

```bash
provenanced tx bank send \
    $(provenanced keys show -a node0 --home build/node0 --keyring-backend test --testnet) \
    $(provenanced keys show -a buyer1 --home build/node0 --keyring-backend test --testnet) \
    200000nhash \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 5000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

```bash
provenanced tx bank send \
    $(provenanced keys show -a node0 --home build/node0 --keyring-backend test --testnet) \
    $(provenanced keys show -a buyer2 --home build/node0 --keyring-backend test --testnet) \
    200000nhash \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 5000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

## Set up orderbook on-chain

```bash
provenanced tx wasm store orderbook.wasm \
    --source "https://github.com/provenance-io/provwasm-examples/tree/main/orderbook" \
    --builder "cosmwasm/rust-optimizer:0.11.3" \
    --instantiate-only-address $(provenanced keys show -a node0 --keyring-backend test --home build/node0 --testnet) \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 5000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Instantiate the contract, setting stablecoin denom required for bids in addition to the price per
hash - for simplicity, we'll just say that `1stablecoin5201` is the price for `1000000000nhash`.

```bash
provenanced tx wasm instantiate 1 '{"bid_denom":"stablecoin5201"}' \
    --admin $(provenanced keys show -a node0 --keyring-backend test --home build/node0 --testnet) \
    --label nhash_orderbook_poc_v1 \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 5000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

## Place ask orders

Sell 10 hash from seller1

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"ask":{"id":"ask-1","price":"1"}}' \
    --amount 10000000000nhash \
    --from seller1 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 5000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Sell 10 hash from asker 2

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"ask":{"id":"ask-2","price":"1"}}' \
    --amount 10000000000nhash \
    --from seller2 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 5000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

## Place bid orders

Buy 5 hash from buyer1

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"bid":{"id":"bid-1","price":"1"}}' \
    --amount 5stablecoin5201 \
    --from buyer1 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 5000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Buy 5 hash from bidder 2

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"bid":{"id":"bid-2","price":"1"}}' \
    --amount 5stablecoin5201 \
    --from buyer2 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 5000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Buy 5 hash from buyer1 for a higher price (should push this to the front of the order book).

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"bid":{"id":"bid-3","price":"2"}}' \
    --amount 10stablecoin5201 \
    --from buyer1 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 5000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

## Query the orderbook

Query bid and ask orders sorted by price-time priority.

```bash
provenanced q wasm contract-state smart \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"get_orderbook":{}}' \
    --testnet -o json | jq
```

## Run a Match

Run the matching algorithm

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"match":{}}' \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 5000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```
