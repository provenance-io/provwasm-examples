# HFT Demo

## HFT

- Shares Infinite @ $1
- Buyer has $100
- Lender 70% LTV on the shares

```text
- Roll for Buy Sell

Buy
- Roll for number of shares (1-100)
- Buy 10 shares

Account
-----------------------
        ($100 - 0 - $0)
Buy 10  ($90 - 10 - $0)
Buy 95  ($0 - 105 - $5)
Sell 20 ($15 - 85 - $0)
```

### Mike Summary

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
