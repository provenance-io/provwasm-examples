# HFT Demo Smart Contract

## Summary

I want to build a proof of concept for high frequency trading on Provenance.

The general pushback on T+instant settlement is that it requires excess capital because of lack of
netting, and prevents the ability to extend credit as trades must be funded in advance of execution.
This is in addition to the concern of speed.

This is misguided on multiple fronts.  Notably, netting happens real time, and credit can be
extended through stablecoin that has a smart contract to manage risk (e.g., what the coin can be
spent on, LTV, etc.). As it relates to speed - the proof is in the pudding, so to speak.

As such, I’d like to build a proof of concept that I think will be extremely powerful in
demonstrating the efficacy of an exchange running on Provenance. The proof of concept would work as
follows.

We establish a trader ("trader") who has a starting stablecoin balance.  Trader will, at random,
buy or sell a single security ("stock") at a price of $1.  Trader will buy or sell with equal
probability between 100 and 1000 shares of stock (uniformly distributed).  Trader instantly settles
with its counterparty. Trader never is short stock.

We also establish a lender ("lender").  Lender will loan trader up to 90% of the value of a stock
purchase, subject to need.  Trader pays back Lender when there is a Lender balance, and trader has
cash.

For example, we might have the following ledger:

Trader -> Lender

Period | Cash | Stock | Loans |
------ | ---- | ----- | ----- |
0      | 100  | 0     | 0     |

Buy 300

Period | Cash | Stock | Loans |
------ | ---- | ----- | ----- |
1      | 0    | 300   | 200   |

Buy 500

Period | Cash | Stock | Loans |
------ | ---- | ----- | ----- |
2      | 0    | 800   | 700   |

Sell 750

Period | Cash | Stock | Loans |
------ | ---- | ----- | ----- |
3      | 50   | 50    | 0     |

Trader can never borrow more than 90% of its stock value (for simplicity, no more than 9X it’s
starting cash balance).  We can optimize the right starting cash for Trader to reduce boundary
conditions where trader can’t purchase.

What we want to show here is the dollar amount of total trades Trader can do, relative to their
cash balance.  This demonstrates that real time netting and smart contract lending can function on
Provenance.  We also want to push as many transactions as possible per second.  This demonstrates
Provenance exchange can scale.

## Smart Contract Demo

The following demonstrates what is required to set up, deploy, and execute the smart contract
portion of the above described demo.

### Blockchain Setup

Clear all current state, install the `provenanced` command, then start a 4-node localnet.

```bash
make clean
make install
make localnet-start
```

### Accounts

An account needs to be set up for the trader.

First, create `trader` account keys

```bash
provenanced keys add trader --home build/node0 --keyring-backend test --testnet
```

If you want to use the trader from this document, use the following to restore the keys locally.

```text
- name: trader
  type: local
  address: tp10etrj2yc8l6sdlc3tct3tzgtdhtj6y7cppm34x
  pubkey: tppub1addwnpepqtrpf5m43749en44jv3cm2nucldw2457q5l3rd27apnn95c98h322xd8u8m
  mnemonic: ""
  threshold: 0
  pubkeys: []


**Important** write this mnemonic phrase in a safe place.
It is the only way to recover your account if you ever forget your password.

odor invite ivory cheese actor wheat mushroom notable broom lucky ensure alarm attract shallow enrich feature good ring unknown deal inner now flat wool
```

Then, fund the `trader` account with `nhash` to pay blockchain fees. We will give the trader
stablecoin funds in a later step.

```bash
provenanced tx bank send \
    $(provenanced keys show -a node0 --home build/node0 --keyring-backend test --testnet) \
    $(provenanced keys show -a trader --home build/node0 --keyring-backend test --testnet) \
    100000000nhash \
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

### Markers

Markers must be created in order to have a supply of demo coins required for buys and sells.

Create the `stock` marker. There is no limit on the amount of coins that can be minted for this
marker. This makes the supply essentially "infinite" for demo purposes.

```bash
provenanced tx marker new 1000000000000demosecurity \
    --type COIN \
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

Create the `stablecoin` marker for the demo.

```bash
provenanced tx marker new 1000000demostablecoin \
    --type COIN \
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

Grant access on the markers to the `node0` account. We need this to give the smart contract the
correct permissions later.

Grants on the `stock` marker.

```bash
provenanced tx marker grant \
    $(provenanced keys show -a node0 --home build/node0 --keyring-backend test --testnet) \
    demosecurity \
    admin,burn,deposit,delete,mint,withdraw \
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

Grants on the `stablecoin` marker.

```bash
provenanced tx marker grant \
    $(provenanced keys show -a node0 --home build/node0 --keyring-backend test --testnet) \
    demostablecoin \
    admin,burn,deposit,delete,mint,withdraw \
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

Finalize the `stock` marker

```bash
provenanced tx marker finalize demosecurity \
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

Finalize the `stablecoin` marker

```bash
provenanced tx marker finalize demostablecoin \
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

Activate the `stock` marker, minting and escrowing the supply

```bash
provenanced tx marker activate demosecurity \
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

Activate the `stablecoin` marker, minting and escrowing the supply

```bash
provenanced tx marker activate demostablecoin \
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

Now, fund the trader account from the `stablecoin` marker.

```bash
provenanced tx marker withdraw demostablecoin \
    100demostablecoin \
    $(provenanced keys show -a trader --home build/node0 --keyring-backend test --testnet) \
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

The `trader` account should now have `nhash` to pay network fees, and `stablecoin` for purchasing
stocks.

```bash
provenanced q bank balances \
    $(provenanced keys show -a trader --home build/node0 --keyring-backend test --testnet) \
    --testnet -o json | jq
```

Example account query output

```json
{
  "balances": [
    {
      "denom": "demostablecoin",
      "amount": "100"
    },
    {
      "denom": "nhash",
      "amount": "100000000"
    }
  ],
  "pagination": {
    "next_key": null,
    "total": "0"
  }
}
```

## Deployment

First, copy the WASM to the Provenance Blockchain project. Then, store the demo smart contract WASM
in provenance.

```bash
provenanced tx wasm store hft.wasm \
    --source "https://github.com/provenance-io/provwasm-examples/tree/main/hft-demo" \
    --builder "cosmwasm/rust-optimizer:0.11.3" \
    --instantiate-only-address $(provenanced keys show -a node0 --keyring-backend test --home build/node0 --testnet) \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 25000nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Instantiate the contract, binding it to the demo markers.

```bash
provenanced tx wasm instantiate 1 '{"security":"demosecurity","stablecoin":"demostablecoin"}' \
    --admin $(provenanced keys show -a node0 --keyring-backend test --home build/node0 --testnet) \
    --label hft_demo_v1 \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 3500nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Add grants to the smart contract for the `stock` marker.

```bash
provenanced tx marker grant \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    demosecurity \
    admin,burn,deposit,delete,mint,withdraw \
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

Add grants to the smart contract for the `stablecoin` marker.

```bash
provenanced tx marker grant \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    demostablecoin \
    admin,burn,deposit,delete,mint,withdraw \
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

Onboard the trader account with the contract (NOTE: trader address value may be different).

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"add_trader":{"address":"tp10etrj2yc8l6sdlc3tct3tzgtdhtj6y7cppm34x"}}' \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 3500nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Query the initial trader state, showing stock balance, stablecoin balance, debt, and loan cap.
(NOTE: trader address value may be different)

```bash
provenanced q wasm contract-state smart \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"get_trader_state":{"address":"tp10etrj2yc8l6sdlc3tct3tzgtdhtj6y7cppm34x"}}' \
    --testnet -o json | jq
 ```

Expected output

```json

  "data": {
    "security": "0",
    "stablecoin": "100",
    "loans": "0",
    "loan_cap": "10000000000"
  }
}
```

## Execution

### Period 1

To execute a buy using `stablecoin`, but still requiring a loan.

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"buy_stock":{"amount":"300"}}' \
    --amount 100demostablecoin \
    --from trader \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 6500nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Query trader state.

```bash
provenanced q wasm contract-state smart \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"get_trader_state":{"address":"tp10etrj2yc8l6sdlc3tct3tzgtdhtj6y7cppm34x"}}' \
    --testnet -o json | jq
 ```

 Expected output

 ```json
{
  "data": {
    "security": "300",
    "stablecoin": "0",
    "loans": "200",
    "loan_cap": "10000000000"
  }
}
```

### Period 2

To execute a buy, but don't send any `stablecoin`, requring more loans to be taken out
(remaining under cap).

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"buy_stock":{"amount":"500"}}' \
    --from trader \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 6500nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Query trader state.

```bash
provenanced q wasm contract-state smart \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"get_trader_state":{"address":"tp10etrj2yc8l6sdlc3tct3tzgtdhtj6y7cppm34x"}}' \
    --testnet -o json | jq
 ```

 Expected output

```json
{
  "data": {
    "security": "800",
    "stablecoin": "0",
    "loans": "700",
    "loan_cap": "10000000000"
  }
}
```

### Period 3

To execute a sell, paying off debt. Result is trader has some `stablecoin` and some `stock`.

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"sell_stock":{"amount":"750"}}' \
    --amount 750demosecurity \
    --from trader \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 6500nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Query trader state.

```bash
provenanced q wasm contract-state smart \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"get_trader_state":{"address":"tp10etrj2yc8l6sdlc3tct3tzgtdhtj6y7cppm34x"}}' \
    --testnet -o json | jq
 ```

Expected output

```json
{
  "data": {
    "security": "50",
    "stablecoin": "50",
    "loans": "0",
    "loan_cap": "10000000000"
  }
}
```

### Period 4

Execute a buy, sending too much `stablecoin`.

```bash
provenanced tx wasm execute \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"buy_stock":{"amount":"10"}}' \
    --amount 20demostablecoin \
    --from trader \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 6500nhash \
    --broadcast-mode block \
    --yes \
    --testnet | jq
```

Query trader state.

```bash
provenanced q wasm contract-state smart \
    tp18vd8fpwxzck93qlwghaj6arh4p7c5n89x8kskz \
    '{"get_trader_state":{"address":"tp10etrj2yc8l6sdlc3tct3tzgtdhtj6y7cppm34x"}}' \
    --testnet -o json | jq
 ```

Should get the overpayment amount of  10 `stablecoin` back

```json
{
  "data": {
    "security": "60",
    "stablecoin": "40",
    "loans": "0",
    "loan_cap": "10000000000"
  }
}
```
