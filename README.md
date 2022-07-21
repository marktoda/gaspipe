# `gaspipe`

A service to estimate gas for multiple dependent Ethereum transactions.

Many user flows in Ethereum nowadays involve sending multiple transactions together (i.e. approve, then deposit). A nice UX would have the user sign them all at once and broadcast them together, ideally confirming in a single block. One challenge with this currently is that it's often difficult to estimate the gas cost for the second transaction given it depends on the first. `gaspipe` solves this problem by executing transactions on a light fork, so each subsequent transaction benefits from the state changes of the last.

# Usage
```bash
$ FORK_URL=https://mainnet.infura.io/v3/<key> gaspipe    
```

## Querying
For example, if you want to estimate an `approve` followed by a `transfer`:

```bash
$ curl -X POST -H "content-type: application/json" localhost:8000/estimate -d '
[
{
    "from": "0x28c6c06298d514db089934071355e5743bf21d60",
    "to": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
    "value": "0",
    "data": "0x095ea7b3000000000000000000000000111111111111111111111111111111111111111100000000000000000000000000000000000000000000000000000000000f4240"
},
{
    "from": "0x1111111111111111111111111111111111111111",
    "to": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
    "value": "0",
    "data": "0x23b872dd00000000000000000000000028c6c06298d514db089934071355e5743bf21d60000000000000000000000000111111111111111111111111111111111111111100000000000000000000000000000000000000000000000000000000000f4240"
}
]'

[{"gas":59963,"reverted":false},{"gas":50056,"reverted": false}] 
```

## Performance
Current average time to estimate the above example: `~1.7s`.
Local fork caching allows for faster times on subsequent calls against the same block (~2.9s on fresh block, ~1.2s on repeat block).
```bash
$ for x in {1..10}; do time curl <estimate>; done

curl -X POST -H "content-type: application/json" localhost:8000/estimate -d   0.00s user 0.00s system 0% cpu 2.936 total
curl -X POST -H "content-type: application/json" localhost:8000/estimate -d   0.01s user 0.00s system 0% cpu 1.223 total
curl -X POST -H "content-type: application/json" localhost:8000/estimate -d   0.00s user 0.01s system 0% cpu 1.186 total
curl -X POST -H "content-type: application/json" localhost:8000/estimate -d   0.00s user 0.01s system 0% cpu 1.180 total
curl -X POST -H "content-type: application/json" localhost:8000/estimate -d   0.00s user 0.00s system 0% cpu 1.286 total
curl -X POST -H "content-type: application/json" localhost:8000/estimate -d   0.01s user 0.00s system 0% cpu 1.195 total
curl -X POST -H "content-type: application/json" localhost:8000/estimate -d   0.01s user 0.00s system 0% cpu 1.206 total
curl -X POST -H "content-type: application/json" localhost:8000/estimate -d   0.00s user 0.01s system 0% cpu 1.176 total
curl -X POST -H "content-type: application/json" localhost:8000/estimate -d   0.00s user 0.01s system 0% cpu 1.207 total
curl -X POST -H "content-type: application/json" localhost:8000/estimate -d   0.00s user 0.01s system 0% cpu 2.867 total
curl -X POST -H "content-type: application/json" localhost:8000/estimate -d   0.01s user 0.00s system 0% cpu 1.213 total
```

# Future Improvements
- [ ] Add an RPC eth_estimateGas style endpoint for simpler migration
- [ ] Improve latency
