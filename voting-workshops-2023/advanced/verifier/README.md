# The verifier backend

This page describes the id verifier backend for the voting workshop example. It takes in proof requests consisting of a statement and a proof for that statement.
The only statement allowed is a non-membership for a list of a single country of residency.
Upon a successful verification, a signature of (account address, country_code) is returned, which must included when casting a vote in the smart contract.

# Supported configuration options

The following parameters are supported
- `node` the URL of the node's GRPC V2 interface, e.g., http://localhost:20000
- `port` the port on which the server will listen for incoming requests
- `log-level` maximum log level (defaults to `debug` if not given)
- `secret-key` path to a binary file with the secret key used for creating the signature.
- `public-key` path to a binary file with the public key used for creating the signature.

All of the above is available by using `--help` to get usage information.

An example to run the verifier with example settings and the public testnet node on would be:
```
cargo run -- --node http://node.testnet.concordium.com:20000
```

# Using the tool

The verifier is a simple server that exposes one endpoint:
 - `POST /prove`.

All of the server state is kept in memory and thus does not survive a restart.

See [src/main.rs](./src/main.rs) for the formats of requests and responses. Both
requests and responses are JSON encoded. The `/prove` endpoint responds with
status `200 OK` and the signature if the proof is acceptable, and with invalid request otherwise.
The requests are handled by handlers in [src/handlers.rs](./src/handlers.rs). 

The server needs access to the node so that it can get the requested credential
from the node during proof validation.
