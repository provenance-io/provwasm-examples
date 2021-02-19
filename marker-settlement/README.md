# Restricted Marker Settlements

__Status__: proof of concept

This smart contract handles transfers for restricted marker settlements. It is intended to be an
actor an exchange ecosystem.

It should be created and used with a sister contract; one that handles settlement transfers using
the bank module.

It is assumed that exchange contracts will instantiate this contract at some point in their
lifecycle.

The exchange can determine the transfers necessary during settlement, and send wasm messages to the
appropriate settlement contract for performing the transfers.

| Ask Type     | Bid Type     | Transfer 1 | Transfer 2 |
| ------------ | ------------ | ---------- | ---------- |
| restricted   | coin         | marker     | bank       |
| restricted   | restricted   | marker     | marker     |
| coin         | coin         | bank       | bank       |
| coin         | restricted   | bank       | marker     |

Where

_restricted_ = funds are backed by a restricted marker

_coin_ = funds are backed by a regular marker
