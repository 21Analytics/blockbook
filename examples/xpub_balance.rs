use bdk::{
    bitcoin::Network,
    wallet::{AddressIndex, Update},
    Wallet,
};

const STOP_GAP: u32 = 50;
const PARALLEL_REQUESTS: usize = 5;

/// Demonstrates the usage of a `Blockbook` client as a
/// data backend for a `bdk` wallet. Here, we sync the
/// balance of a watch-only xpub wallet.
///
/// For more involved examples of how to use `bdk` wallets, see their
/// [example crates](https://github.com/bitcoindevkit/bdk/tree/3569acca0b3f3540e1f1a2278794eac4642a05e4/example-crates).
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let blockbook_server = std::env::var("BLOCKBOOK_SERVER")
        .expect("please set the `BLOCKBOOK_SERVER` environment variable");

    // https://www.blockchain.com/explorer/assets/btc/xpub/xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz
    let external_descriptor = "pkh(xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz/0/*)";
    let internal_descriptor = "pkh(xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz/1/*)";

    let mut wallet = Wallet::new_no_persist(
        external_descriptor,
        Some(internal_descriptor),
        Network::Bitcoin,
    )?;

    let address = wallet.get_address(AddressIndex::New);
    println!("Generated Address: {address}");

    let balance = wallet.get_balance();
    println!("Wallet balance before syncing: {} sats", balance.total());

    println!("Syncing...");
    let client = blockbook::Client::new(blockbook_server.parse().unwrap())
        .await
        .unwrap();

    let (update_graph, last_active_indices) = client
        .scan_txs_with_keychains(
            wallet.spks_of_all_keychains(),
            &Network::Bitcoin,
            None,
            None,
            STOP_GAP,
            PARALLEL_REQUESTS,
        )
        .await?;
    let prev_tip = wallet.latest_checkpoint();
    let missing_heights = update_graph.missing_heights(wallet.local_chain());
    let chain_update = client.update_local_chain(prev_tip, missing_heights).await?;
    let update = Update {
        last_active_indices,
        graph: update_graph,
        chain: Some(chain_update),
    };
    wallet.apply_update(update)?;
    wallet.commit()?;

    let balance = wallet.get_balance();
    println!("Wallet balance after syncing: {} sats", balance.total());
    Ok(())
}
