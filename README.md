stale
=====
Guardrail for onchain agents. Before an agent acts on a Chainlink price, stale
checks whether that price is still fresh. If it is stale, expired, or
not-yet-valid, it returns BLOCK and a notify message. It never sends a
transaction. It never invents a feed. It fails closed on every path that
could let a stale price through.


Install
-------
```
npm install @ramprasad4121/stale
```

(or)
```
git clone https://github.com/Ramprasad4121/stale.git && cd stale && npm install # secondary: local dev path
```

Use
---
CLI:

npx stale --rpc $RPC_URL --maxAge 3600
npx stale --rpc $RPC_URL --maxAge 60 --feed 0xF4030086522a5bEEa4988F8cA5B36dbC97BeE88c --amount 0.5

Library:

    import { checkPrice } from "@ramprasad4121/stale";

    const r = await checkPrice({
      rpc: process.env.RPC_URL,
      feed: "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419",
      maxAgeSeconds: 3600,
    });

    if (r.decision === "BLOCK") {
      // do not act
    }

--rpc and --maxAge are required.
Optional: --amount 0.5  --feed <allowlisted proxy>  --json
Exit 0 = ALLOW. Exit 1 = BLOCK.

Supported feeds (Ethereum mainnet, default ETH/USD): ETH/USD
0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419 and BTC/USD
0xF4030086522a5bEEa4988F8cA5B36dbC97BeE88c —
https://docs.chain.link/data-feeds/price-feeds/addresses?network=ethereum

maxAge is your policy. ETH/USD heartbeat is about 45-50 minutes.
--maxAge 60 will usually BLOCK even when the feed is healthy.

What BLOCK means
----------------
Do not act. Reasons include: stale or future timestamp, updatedAt == 0,
answer <= 0, incomplete round (answeredInRound < roundId), wrong chainId
for the feed, unknown feed, bad RPC, or invalid input.

Source: https://docs.chain.link/data-feeds/api-reference

Contribute
----------
See CONTRIBUTING.md.

 