# Rust Blockbook client

[![Crates.io](https://img.shields.io/crates/v/blockbook.svg)](https://crates.io/crates/blockbook)
[![Documentation](https://docs.rs/blockbook/badge.svg)](https://docs.rs/blockbook/)

`blockbook` is a Rust client for the [Blockbook block explorer](https://github.com/trezor/blockbook).

It provides both REST and WebSocket clients to access Blockbook APIs in a type-safe manner.

## Usage example

```rust
#[tokio::main]
async fn main() {
    let client = blockbook::Blockbook::new("https://myblockbook.com".parse().unwrap());

    // query the Genesis block hash
    let genesis_hash = client
        .block_hash(blockbook::Height::from_consensus(0).unwrap())
        .await?;
    assert_eq!(
        genesis_hash.to_string(),
        "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f"
    );

    // query the full block
    let genesis = client.block_by_hash(genesis_hash).await?;
    assert_eq!(genesis.previous_block_hash, None);

    // inspect the first coinbase transaction
    let tx = genesis.txs.get(0).unwrap();
    assert!((tx.vout.get(0).unwrap().value.to_btc() - 50.0).abs() < f64::EPSILON);
}
```

See the [crate documentation](https://docs.rs/blockbook) for more examples.

## Supported Blockbook version

The currently supported version of Blockbook is [`0.4.0`](https://github.com/trezor/blockbook/releases/tag/v0.4.0).

## Supported currencies

Currently, `blockbook` only provides Bitcoin-specific APIs.

## Running the tests

All tests need a `BLOCKBOOK_SERVER` environment variable set:

```bash
BLOCKBOOK_SERVER=myblockbook.com cargo test -- --include-ignored
```

## Authors

This crate is developed and maintained by [21 Analytics](https://21analytics.ch).

## License

This project is licensed under the MIT license.
