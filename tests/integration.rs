#[tokio::test]
async fn blockbook() {
    blockbook::Blockbook::new(url::Url::parse("https://btc1.trezor.io").unwrap());
}
