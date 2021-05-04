# Hash Order Book Matching Example

This is an example order book that explores matching buys and sells on-chain.

For simplicity, sell orders must be `nhash` in increments of 1,000,000,000. Buy order denom can be
specified during instantiation, but it must be backed by an unrestricted marker.

The matching algorithm used in this example is __Price-Time-Priority/FIFO__

NOTE: This is not truly a decentralized orderbook on blockchain - it requires a privileged account
to run the matching algorithm (and pay fees) no more than once per block. This admin account is
prohibited from placing buy/sell orders. However, using an admin account does not prevent orderbook
manipulation - the organization controlling the match executions could have a separate address used
for placing buy/sell orders. In theory, they could "watch" the orderbook, then execute the match
when their buy/sell account has orders in an optimal position. In order for this to be truly
decentralized, there needs to be a begin/end block hook (or something similar) that executes
matching - which is not currently available to CosmWasm contracts.

## TODO

- Cancel buy
- Cancel sell
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

Accounts needs to be set up for buyers and sellers

Buyer1

```bash
provenanced keys add buyer1 --home build/node0 --keyring-backend test --testnet


- name: buyer1
  type: local
  address: tp1kafgjq4kefv2f5lt85hvhqxcl460juxfxqz58k
  pubkey: tppub1addwnpepqvrzxjxxteudpegj0kufk4vkga9get0vzphhdhwz2fe08wsneyz3y42weq7
  mnemonic: ""
  threshold: 0
  pubkeys: []


**Important** write this mnemonic phrase in a safe place.
It is the only way to recover your account if you ever forget your password.

orphan match iron roast toss rhythm divert silk festival enlist school tent grant save boring type pretty setup monitor midnight cereal spoil jungle width

```

Buyer2

```bash
provenanced keys add buyer2 --home build/node0 --keyring-backend test --testnet

- name: buyer2
  type: local
  address: tp1d8efdneqc5yw3xu6mmjduq7z75k3v4nz7x9jgx
  pubkey: tppub1addwnpepqvxk3amumtqjus7kwkg2nyajr05z5duzrq3u9q3d9s085v8yecl5v7d7xz2
  mnemonic: ""
  threshold: 0
  pubkeys: []


**Important** write this mnemonic phrase in a safe place.
It is the only way to recover your account if you ever forget your password.

away icon useless girl matrix heart stone vehicle spoil never minor stock ethics space above jar law attitude youth plug identify coyote rebel merit
```

Seller1

```bash
provenanced keys add seller1 --home build/node0 --keyring-backend test --testnet

- name: seller1
  type: local
  address: tp1uhcvjyr437ur24yhtv228yrurt9zsthpv40gs6
  pubkey: tppub1addwnpepqt0n5n5cmskxhps2fj6tquaju7s5nffawfe8jfq9z3rf277ztlk628thpjr
  mnemonic: ""
  threshold: 0
  pubkeys: []


**Important** write this mnemonic phrase in a safe place.
It is the only way to recover your account if you ever forget your password.

guitar million cloud flee lyrics property where course clean curious quality swing exist toilet equal scan cup garbage economy moral basic eternal baby gift
```

Seller2

```bash
provenanced keys add seller2 --home build/node0 --keyring-backend test --testnet

- name: seller2
  type: local
  address: tp1n0apzz2k9fda3hzlqqmu70kkkfcgc5sqqhe777
  pubkey: tppub1addwnpepq2cvkg4dgt0fxh2u345qu3u4emr9jne3hxsh0h79kdf39ekt0s05yuxfvf6
  mnemonic: ""
  threshold: 0
  pubkeys: []


**Important** write this mnemonic phrase in a safe place.
It is the only way to recover your account if you ever forget your password.

mammal trip dentist account glory monster picnic give rate wear stool jump bubble trial virtual marine vintage cattle drink congress sting device click sing
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

Instantiate the contract, setting stablecoin denom required for buys in addition to the price per
hash - for simplicity, we'll just say that `1stablecoin5201` is the price for `1000000000nhash`.

```bash
provenanced tx wasm instantiate 1 '{"buy_denom":"stablecoin5201"}' \
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

## Place sell orders

Sell 10 hash from seller1

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
    --fees 5000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Sell 10 hash from seller 2

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"sell":{"id":"sell-2","price":"1"}}' \
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

## Place buy orders

Buy 5 hash from buyer1

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
    --fees 5000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Buy 5 hash from buyer 2

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
    --fees 5000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Buy 5 hash from buyer1 for a higher price (should push this to the front of the order book).

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"buy":{"id":"buy-3","price":"2"}}' \
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

Query buy and sell orders sorted by price-time priority.

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

Example output from match

```json
{
  "height": "129",
  "txhash": "3B905186CA5E66E4C9647727F0DB6C1ED31AA87BC447FF2C45F118BF0E1935D7",
  "codespace": "",
  "code": 0,
  "data": "0A090A0765786563757465",
  "raw_log": "<snip>",
  "logs": [
    {
      "msg_index": 0,
      "log": "",
      "events": [
        {
          "type": "message",
          "attributes": [
            {
              "key": "action",
              "value": "execute"
            },
            {
              "key": "module",
              "value": "wasm"
            },
            {
              "key": "signer",
              "value": "tp1nxmnst765pkevx7g8h2msmlkrpq25qtqluxatr"
            },
            {
              "key": "contract_address",
              "value": "tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz"
            }
          ]
        },
        {
          "type": "transfer",
          "attributes": [
            {
              "key": "recipient",
              "value": "tp1uhcvjyr437ur24yhtv228yrurt9zsthpv40gs6"
            },
            {
              "key": "sender",
              "value": "tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz"
            },
            {
              "key": "amount",
              "value": "10stablecoin5201"
            },
            {
              "key": "recipient",
              "value": "tp1kafgjq4kefv2f5lt85hvhqxcl460juxfxqz58k"
            },
            {
              "key": "sender",
              "value": "tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz"
            },
            {
              "key": "amount",
              "value": "5000000000nhash"
            },
            {
              "key": "recipient",
              "value": "tp1uhcvjyr437ur24yhtv228yrurt9zsthpv40gs6"
            },
            {
              "key": "sender",
              "value": "tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz"
            },
            {
              "key": "amount",
              "value": "5000000000nhash"
            },
            {
              "key": "recipient",
              "value": "tp1n0apzz2k9fda3hzlqqmu70kkkfcgc5sqqhe777"
            },
            {
              "key": "sender",
              "value": "tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz"
            },
            {
              "key": "amount",
              "value": "5stablecoin5201"
            },
            {
              "key": "recipient",
              "value": "tp1d8efdneqc5yw3xu6mmjduq7z75k3v4nz7x9jgx"
            },
            {
              "key": "sender",
              "value": "tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz"
            },
            {
              "key": "amount",
              "value": "5000000000nhash"
            },
            {
              "key": "recipient",
              "value": "tp1n0apzz2k9fda3hzlqqmu70kkkfcgc5sqqhe777"
            },
            {
              "key": "sender",
              "value": "tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz"
            },
            {
              "key": "amount",
              "value": "5stablecoin5201"
            },
            {
              "key": "recipient",
              "value": "tp1kafgjq4kefv2f5lt85hvhqxcl460juxfxqz58k"
            },
            {
              "key": "sender",
              "value": "tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz"
            },
            {
              "key": "amount",
              "value": "5000000000nhash"
            }
          ]
        },
        {
          "type": "wasm",
          "attributes": [
            {
              "key": "contract_address",
              "value": "tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz"
            },
            {
              "key": "match",
              "value": "buy:buy-3,sell:sell-1"
            },
            {
              "key": "match",
              "value": "buy:buy-2,sell:sell-2"
            },
            {
              "key": "match",
              "value": "buy:buy-1,sell:sell-2"
            }
          ]
        }
      ]
    }
  ],
  "info": "",
  "gas_wanted": "201740",
  "gas_used": "200227",
  "tx": null,
  "timestamp": ""
}
```
