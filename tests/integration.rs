fn blockbook() -> blockbook::Blockbook {
    blockbook::Blockbook::new(url::Url::parse("https://btc1.trezor.io").unwrap())
}

#[tokio::test]
async fn test_block_hash() {
    let hash = blockbook().block_hash(763_672).await.unwrap().block_hash;
    assert_eq!(
        hash,
        "00000000000000000002be1ecb5125765125a8d9836a7f2cefbecf646fa2674e"
    );
}
