# Order Book Matching Example

This is an example order book that explores matching buys and sells on-chain.

For simplicity, sell orders must be `nhash`. Buy order denom can be specified during instantiation,
however, it must represent an unrestricted marker (ie some stablecoin).

The matching algorithm used in this example is __Price-Time-Priority/FIFO__

## Blockchain Setup

Clear all current state, install the `provenanced` command, then start a 4-node localnet.

```bash
make clean
make install
make localnet-start
```

## Accounts

Accounts needs to be set up for buyers and sellers

Buyer1

```bash
provenanced keys add buyer1 --home build/node0 --keyring-backend test --testnet
```

Buyer2

```bash
provenanced keys add buyer2 --home build/node0 --keyring-backend test --testnet
```

Seller1

```bash
provenanced keys add seller1 --home build/node0 --keyring-backend test --testnet
```

Seller2

```bash
provenanced keys add seller2 --home build/node0 --keyring-backend test --testnet
```

## Funding Sellers

Fund the seller accounts with a bunch of `nhash`

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

A marker must be created in order to have a supply of stablecoin required for buying nhash.

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
    admin,burn,deposit,delete,mint,transfer,withdraw \
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
    --fees 2000nhash \
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
    --fees 2000nhash \
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
    --fees 2000nhash \
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
    --fees 2000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

## Set up orderbook on-chain

```bash
provenanced tx wasm store orderbook.wasm \
    --source "https://github.com/provenance-io/provwasm-examples/tree/main/orderbook" \
    --builder "cosmwasm/rust-optimizer:0.11.0" \
    --instantiate-only-address $(provenanced keys show -a node0 --keyring-backend test --home build/node0 --testnet) \
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

Instantiate the contract, setting stablecoin denom required for buys in addition to the price per
hash - for simplicity, we say that `1stablecoin5201` is the price for `100000000nhash`.

```bash
provenanced tx wasm instantiate 1 '{"buy_denom":"stablecoin5201"}' \
    --admin $(provenanced keys show -a node0 --keyring-backend test --home build/node0 --testnet) \
    --label orderbook_v1 \
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

## Place sell orders

Sell 10 hash

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"sell":{"id":"sell-1","price":"1"}}' \
    --amount 10000000000nhash \
    --from seller1 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 2000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Sell 13 hash

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"sell":{"id":"sell-2","price":"1"}}' \
    --amount 13000000000nhash \
    --from seller2 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 2000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

## Place buy orders

Buy 5 hash

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"buy":{"id":"buy-1","price":"1"}}' \
    --amount 5stablecoin5201 \
    --from buyer1 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 2000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Buy 5 hash

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"buy":{"id":"buy-2","price":"1"}}' \
    --amount 5stablecoin5201 \
    --from buyer2 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 2000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Buy 10 hash

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"buy":{"id":"buy-3","price":"1"}}' \
    --amount 10stablecoin5201 \
    --from buyer1 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 2000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

## Query orderbook

Query buy orders in price-time sorted priority.

```bash
provenanced q wasm contract-state smart \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"get_buy_orders":{}}' \
    --testnet -o json | jq
```

Query sell orders in price-time sorted priority.

```bash
provenanced q wasm contract-state smart \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"get_sell_orders":{}}' \
    --testnet -o json | jq
```

## Run match action

TODO
