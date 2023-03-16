## Solana Devtools RPC

### Client Headers

There is a `HttpSenderWithHeaders` struct that allows for default headers
to be passed in.

This is useful for example when using authenticated RPC providers.

Example Usage:
```
let rpc_addr = "http://localhost:8899";

// Set some headers
let mut default_headers = HeaderMap::new();
default_headers.insert("foo", HeaderValue::from_str("bar").unwrap());

let sender = HttpSenderWithHeaders::new(
    rpc_addr,
    Some(default_headers)
);
let rpc_client = RpcClient::new_sender(sender, Default::default());
// make requests like usual.
```
