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

We establish a trader (“trader”) who has a starting stablecoin balance.  Trader will, at random,
buy or sell a single security (“stock”) at a price of $1.  Trader will buy or sell with equal
probability between 100 and 1000 shares of stock (uniformly distributed).  Trader instantly settles
with its counterparty. Trader never is short stock.

We also establish a lender (“lender”).  Lender will loan trader up to 90% of the value of a stock
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

Let me know if this makes sense. It would be very powerful to have the ability to run this demo,
and demonstrate to entities like DTCC, brokers and Shareworks.

## Blockchain Setup

### Start

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
    --testnet
```

### Markers

Markers must be created in order to have a supply of demo coins required for buys and sells.

Create the restricted `stock` marker (no direct account-to-account sends for stock). There is no
limit on the amount of coins that can be minted for this marker. This makes the supply "infinite"
for demo purposes. A typical stock would have a fixed supply.

```bash
provenanced tx marker new 1000000000000demosecurity \
    --type RESTRICTED \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 5000nhash \
    --broadcast-mode block \
    --yes \
    --testnet
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
    --testnet
```

Grant access on the markers to the `node0` account. We need this to give the smart contract the
correct permissions later.

Grants on the `stock` marker.

```bash
provenanced tx marker grant \
    $(provenanced keys show -a node0 --home build/node0 --keyring-backend test --testnet) \
    demosecurity \
    admin,burn,mint \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 5000nhash \
    --broadcast-mode block \
    --yes \
    --testnet
```

Grants on the `stablecoin` marker.

```bash
provenanced tx marker grant \
    $(provenanced keys show -a node0 --home build/node0 --keyring-backend test --testnet) \
    demostablecoin \
    admin,burn,mint,withdraw \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto \
    --fees 5000nhash \
    --broadcast-mode block \
    --yes \
    --testnet
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
    --testnet
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
    --testnet
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
    --testnet
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
    --testnet
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
    --testnet
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

TODO

## Execution

TODO
