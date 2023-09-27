# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

- Renamed `blockbook::websocket::Error`'s `WebsocketError` variant to `Websocket`.
- Allowed a `blockbook::TransactionSpecific` to be converted into a `bitcoin::Transaction`.
- Renamed both rest and websocket clients from `Blockbook` to `Client`.
- Renamed `blockbook::Client::send_transaction` to `blockbook::Client::broadcast_transaction`.
- Bumped `tokio-tungstenite` to version 0.20.0 that is not vulnerable to
  https://rustsec.org/advisories/RUSTSEC-2023-0052 via the `webpki` crate.
- Added support for addresses without any transactions.
- Bumped the minimum supported Blockbook version to [commit `95ee9b5b`](https://github.com/trezor/blockbook/commit/95ee9b5b).
- Bumped `bitcoin` to version 0.31.
- Adopted the block and transaction version to the corresponding `bitcoin` type.
- Added a `bdk` feature that gates functionality to use Blockbook as a data provider for
  the popular [`bdk`](https://crates.io/crates/bdk) wallet library.

## 0.1.0

- Initial release of this crate.
