use blockbook::{
    hashes::{self, hex::FromHex},
    websocket::{Blockbook, Info},
    Address, AddressBlockVout, AddressFilter, AddressInfo, AddressInfoBasic, AddressInfoDetailed,
    AddressInfoPaging, Amount, Asset, Block, BlockHash, BlockTransaction, BlockVin, BlockVout,
    Chain, Currency, Height, LockTime, NetworkUnchecked, OpReturn, ScriptBuf, ScriptPubKey,
    ScriptPubKeyType, ScriptSig, Sequence, Ticker, TickersList, Time, Token, Transaction,
    TransactionSpecific, Tx, TxDetail, Txid, Utxo, Version, Vin, VinSpecific, Vout, VoutSpecific,
    Witness, XPubInfo, XPubInfoBasic,
};
use std::str::FromStr;

fn blockbook() -> blockbook::Blockbook {
    blockbook::Blockbook::new(
        format!("https://{}", std::env::var("BLOCKBOOK_SERVER").unwrap())
            .parse()
            .unwrap(),
    )
}

async fn blockbook_ws() -> Blockbook {
    Blockbook::new(
        format!(
            "wss://{}/websocket",
            std::env::var("BLOCKBOOK_SERVER").unwrap()
        )
        .parse()
        .unwrap(),
    )
    .await
    .unwrap()
}

#[ignore]
#[tokio::test]
async fn test_status() {
    let status = blockbook().status().await.unwrap();

    assert_eq!(status.blockbook.coin, Asset::Bitcoin);
    assert_eq!(status.blockbook.decimals, 8);
    assert_eq!(
        status.blockbook.version,
        semver::Version::parse("0.4.0").unwrap()
    );
    assert_eq!(status.backend.chain, Chain::Main);
    assert_eq!(status.backend.protocol_version, "70016");
}

#[ignore]
#[tokio::test]
async fn test_block_hash() {
    let hash = blockbook()
        .block_hash(Height::from_consensus(763_672).unwrap())
        .await
        .unwrap();
    assert_eq!(
        hash.as_ref(),
        [
            0x4e, 0x67, 0xa2, 0x6f, 0x64, 0xcf, 0xbe, 0xef, 0x2c, 0x7f, 0x6a, 0x83, 0xd9, 0xa8,
            0x25, 0x51, 0x76, 0x25, 0x51, 0xcb, 0x1e, 0xbe, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ],
    );
}

#[allow(clippy::too_many_lines)]
#[ignore]
#[tokio::test]
async fn test_tx() {
    let txid = "b0714235addd08daf83b979aa35cc9ed7558efb8327b86b4d3ccacd8b0482ae1"
        .parse()
        .unwrap();
    let tx = blockbook().transaction(txid).await.unwrap();
    let expected_tx = Transaction {
        txid,
        version: 2,
        lock_time: None,
        script: ScriptBuf::from_hex(
            "0200000000010227fa15587fdefa0e4bddef8297e8e309478b6ecad3c79ccc4f11e0bf5a0ee4a60200000000ffffffff5a43b6f57bf76889924a806ae1cf2124de949c3f5acbeade82489f213837dde00000000000ffffffff034078f30100000000160014fff48015913acb35add9b3c74b5a6f76f2d145caf6c19c0000000000160014e1a8d8ab85131ec8a9521ad2e7daf16bcf3fe9e2187e36030000000016001431ac0b37f2dfa0a34c65e57c209361a916feb62d02483045022100e6bc8783be4d00222da4684b495c0e38578c72bc72d74321b776e777f430b5be02203dce27e811f0cb0b9ebad0d11d541a83344c985ec37a0273993caf582988c0a301210285b14271e50491ac26111dd42a6d9004f06a8e77355dac918c2fe7b1a7526ff002473044022036563b247efe66f50f453a6417d03bca152ad70913d7b69b29d7abcb602dd389022033f841a69c985ba457fb1c41a533fb0bce4b68a3bd42fdec60a89ab66623995901210285b14271e50491ac26111dd42a6d9004f06a8e77355dac918c2fe7b1a7526ff000000000"
            )
            .unwrap(),
        block_hash: Some(BlockHash::from_raw_hash(
            hashes::sha256d::Hash::from_str(
                "00000000000000000006b7e2a7110c174f21633adbe955c8f86f36699bba6716",
            )
            .unwrap(),
        )),
        size: 402,
        vsize: 240,
        block_height: Some(Height::from_consensus(765_165).unwrap()),
        confirmations: tx.confirmations,
        block_time: Time::from_consensus(1_669_723_092).unwrap(),
        value: Amount::from_sat(96_909_390),
        value_in: Amount::from_sat(96_935_907),
        fees: Amount::from_sat(26517),
        vin: vec![
            Vin {
                txid: "a6e40e5abfe0114fcc9cc7d3ca6e8b4709e3e89782efdd4b0efade7f5815fa27"
                    .parse()
                    .unwrap(),
                sequence: Some(Sequence(4_294_967_295)),
                n: 0,
                vout: Some(2),
                addresses: vec!["bc1qxxkqkdljm7s2xnr9u47zpymp4yt0ad3df4hudm"
                    .parse::<Address<NetworkUnchecked>>()
                    .unwrap()
                    .assume_checked()],
                is_address: true,
                value: Amount::from_sat(59_231_084),
            },
            Vin {
                txid: "e0dd3738219f4882deeacb5a3f9c94de2421cfe16a804a928968f77bf5b6435a"
                    .parse()
                    .unwrap(),
                vout: None,
                sequence: Some(Sequence(4_294_967_295)),
                n: 1,
                addresses: vec!["bc1qxxkqkdljm7s2xnr9u47zpymp4yt0ad3df4hudm"
                    .parse::<Address<NetworkUnchecked>>()
                    .unwrap()
                    .assume_checked()],
                is_address: true,
                value: Amount::from_sat(37_704_823),
            },
        ],
        vout: vec![
            Vout {
                value: Amount::from_sat(32_733_248),
                n: 0,
                spent: Some(true),
                script: ScriptBuf::from_hex("0014fff48015913acb35add9b3c74b5a6f76f2d145ca")
                    .unwrap(),
                addresses: vec!["bc1qll6gq9v38t9nttwek0r5kkn0wmedz3w2gshe0a"
                    .parse::<Address<NetworkUnchecked>>()
                    .unwrap()
                    .assume_checked()],
                is_address: true,
            },
            Vout {
                value: Amount::from_sat(10_273_270),
                n: 1,
                spent: Some(true),
                script: ScriptBuf::from_hex("0014e1a8d8ab85131ec8a9521ad2e7daf16bcf3fe9e2")
                    .unwrap(),
                addresses: vec!["bc1qux5d32u9zv0v322jrtfw0kh3d08nl60z8q964g"
                    .parse::<Address<NetworkUnchecked>>()
                    .unwrap()
                    .assume_checked()],
                is_address: true,
            },
            Vout {
                value: Amount::from_sat(53_902_872),
                n: 2,
                spent: Some(true),
                script: ScriptBuf::from_hex("001431ac0b37f2dfa0a34c65e57c209361a916feb62d")
                    .unwrap(),
                addresses: vec!["bc1qxxkqkdljm7s2xnr9u47zpymp4yt0ad3df4hudm"
                    .parse::<Address<NetworkUnchecked>>()
                    .unwrap()
                    .assume_checked()],
                is_address: true,
            },
        ],
    };
    assert_eq!(tx, expected_tx);
    let mut client = blockbook_ws().await;
    assert_eq!(client.transaction(txid).await.unwrap(), expected_tx);
}

#[ignore]
#[tokio::test]
async fn test_lock_time() {
    let tx = blockbook()
        .transaction(
            "bd99f123432e23aa8b88af0e9f701a4d6c8f0638dc133a14c7ccf57fb06596ac"
                .parse()
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tx.lock_time, Some(Height::from_consensus(777_536).unwrap()));
}

#[ignore]
#[tokio::test]
async fn test_sequence() {
    let tx = blockbook()
        .transaction(
            "bd99f123432e23aa8b88af0e9f701a4d6c8f0638dc133a14c7ccf57fb06596ac"
                .parse()
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        tx.vin.get(0).unwrap().sequence,
        Some(Sequence(4_294_967_293))
    );
    let tx = blockbook()
        .transaction(
            "c8d7b00135b9bd03055a8f47851eafae747b759b4608bd9f35e85b3285185679"
                .parse()
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tx.vin.get(0).unwrap().sequence, None);
}

#[allow(clippy::too_many_lines)]
#[ignore]
#[tokio::test]
async fn test_tx_specific() {
    let txid = "b0714235addd08daf83b979aa35cc9ed7558efb8327b86b4d3ccacd8b0482ae1"
        .parse()
        .unwrap();
    let tx = blockbook().transaction_specific(txid).await.unwrap();
    let expected_tx = TransactionSpecific {
        txid,
        version: 2,
        script: ScriptBuf::from_hex(
            "0200000000010227fa15587fdefa0e4bddef8297e8e309478b6ecad3c79ccc4f11e0bf5a0ee4a60200000000ffffffff5a43b6f57bf76889924a806ae1cf2124de949c3f5acbeade82489f213837dde00000000000ffffffff034078f30100000000160014fff48015913acb35add9b3c74b5a6f76f2d145caf6c19c0000000000160014e1a8d8ab85131ec8a9521ad2e7daf16bcf3fe9e2187e36030000000016001431ac0b37f2dfa0a34c65e57c209361a916feb62d02483045022100e6bc8783be4d00222da4684b495c0e38578c72bc72d74321b776e777f430b5be02203dce27e811f0cb0b9ebad0d11d541a83344c985ec37a0273993caf582988c0a301210285b14271e50491ac26111dd42a6d9004f06a8e77355dac918c2fe7b1a7526ff002473044022036563b247efe66f50f453a6417d03bca152ad70913d7b69b29d7abcb602dd389022033f841a69c985ba457fb1c41a533fb0bce4b68a3bd42fdec60a89ab66623995901210285b14271e50491ac26111dd42a6d9004f06a8e77355dac918c2fe7b1a7526ff000000000"
            )
            .unwrap(),
        wtxid: "5eb2c13ef559e9aee53e803e6a8abcbfc7bb60830e2bb7fec1437f98f463e889"
            .parse()
            .unwrap(),
        size: 402,
        time: Time::from_consensus(1_669_723_092).unwrap(),
        vsize: 240,
        weight: 957,
        blockhash: "00000000000000000006b7e2a7110c174f21633adbe955c8f86f36699bba6716"
            .parse()
            .unwrap(),
        blocktime: Time::from_consensus(1_669_723_092).unwrap(),
        confirmations: tx.confirmations,
        locktime: LockTime::ZERO,
        vin: vec![
            VinSpecific {
                txid: "a6e40e5abfe0114fcc9cc7d3ca6e8b4709e3e89782efdd4b0efade7f5815fa27"
                    .parse()
                    .unwrap(),
                sequence: Sequence(4_294_967_295),
                vout: 2,
                tx_in_witness: Some(Witness::from_slice(&[
                    Vec::from_hex(
                        "3045022100e6bc8783be4d00222da4684b495c0e38578c72bc72d74321b776e777f430b5be02203dce27e811f0cb0b9ebad0d11d541a83344c985ec37a0273993caf582988c0a301"
                    )
                    .unwrap(),
                    Vec::from_hex(
                        "0285b14271e50491ac26111dd42a6d9004f06a8e77355dac918c2fe7b1a7526ff0",
                    )
                    .unwrap(),
                ])),
                script_sig: blockbook::ScriptSig::default(),
            },
            VinSpecific {
                txid: "e0dd3738219f4882deeacb5a3f9c94de2421cfe16a804a928968f77bf5b6435a"
                    .parse()
                    .unwrap(),
                sequence: Sequence(4_294_967_295),
                vout: 0,
                tx_in_witness: Some(Witness::from_slice(&[
                    Vec::from_hex(
                        "3044022036563b247efe66f50f453a6417d03bca152ad70913d7b69b29d7abcb602dd389022033f841a69c985ba457fb1c41a533fb0bce4b68a3bd42fdec60a89ab66623995901"
                    )
                    .unwrap(),
                    Vec::from_hex(
                        "0285b14271e50491ac26111dd42a6d9004f06a8e77355dac918c2fe7b1a7526ff0",
                    )
                    .unwrap(),
                ])),
                script_sig: blockbook::ScriptSig::default(),
            },
        ],
        vout: vec![
            VoutSpecific {
                value: Amount::from_sat(32_733_248),
                n: 0,
                script_pub_key: ScriptPubKey {
                    address: "bc1qll6gq9v38t9nttwek0r5kkn0wmedz3w2gshe0a"
                        .parse::<Address<NetworkUnchecked>>()
                        .unwrap()
                        .assume_checked(),
                    asm: "0 fff48015913acb35add9b3c74b5a6f76f2d145ca".into(),
                    desc: Some("addr(bc1qll6gq9v38t9nttwek0r5kkn0wmedz3w2gshe0a)#k54tyxd5".into()),
                    script: ScriptBuf::from_hex("0014fff48015913acb35add9b3c74b5a6f76f2d145ca")
                        .unwrap(),
                    r#type: ScriptPubKeyType::WitnessV0PubKeyHash,
                },
            },
            VoutSpecific {
                value: Amount::from_sat(10_273_270),
                n: 1,
                script_pub_key: ScriptPubKey {
                    address: "bc1qux5d32u9zv0v322jrtfw0kh3d08nl60z8q964g"
                        .parse::<Address<NetworkUnchecked>>()
                        .unwrap()
                        .assume_checked(),
                    asm: "0 e1a8d8ab85131ec8a9521ad2e7daf16bcf3fe9e2".into(),
                    desc: Some("addr(bc1qux5d32u9zv0v322jrtfw0kh3d08nl60z8q964g)#f20g3jce".into()),
                    script: ScriptBuf::from_hex("0014e1a8d8ab85131ec8a9521ad2e7daf16bcf3fe9e2")
                        .unwrap(),
                    r#type: ScriptPubKeyType::WitnessV0PubKeyHash,
                },
            },
            VoutSpecific {
                value: Amount::from_sat(53_902_872),
                n: 2,
                script_pub_key: ScriptPubKey {
                    address: "bc1qxxkqkdljm7s2xnr9u47zpymp4yt0ad3df4hudm"
                        .parse::<Address<NetworkUnchecked>>()
                        .unwrap()
                        .assume_checked(),
                    asm: "0 31ac0b37f2dfa0a34c65e57c209361a916feb62d".into(),
                    desc: Some("addr(bc1qxxkqkdljm7s2xnr9u47zpymp4yt0ad3df4hudm)#mxf8kxkq".into()),
                    script: ScriptBuf::from_hex("001431ac0b37f2dfa0a34c65e57c209361a916feb62d")
                        .unwrap(),
                    r#type: ScriptPubKeyType::WitnessV0PubKeyHash,
                },
            },
        ],
    };
    assert_eq!(tx, expected_tx);

    assert_eq!(
        blockbook_ws()
            .await
            .transaction_specific(txid)
            .await
            .unwrap(),
        expected_tx
    );
}

#[ignore]
#[tokio::test]
async fn test_tx_specific_pre_segwit() {
    #![allow(clippy::similar_names)]
    let txid = "0c5cb51f39ecb826cd477d94576abde1d2b6ef1b2e0ac7b9cea5d5ab28aba902";
    let wtxid = txid.parse().unwrap();
    let txid = txid.parse().unwrap();
    let tx = blockbook().transaction_specific(txid).await.unwrap();
    let expected_tx = TransactionSpecific {
        txid,
        version: 1,
        script: ScriptBuf::from_hex(
            "01000000013cdefb50d22666b59b24f047b019e09a2c077ad0fb8febda33a5e0bad45990e2000000006a47304402202015dfc5b5d9030f9538c1f6e0b99fe8dbf46260044e45ad2f883744292af09b0220066353c0d19f9734278ba7072fa8b3ba1c1c30bdd583721439b8ee375a098ad8012103de2010f23c4eda698d373cfc8f7ecd576fbb4e40f67a8634ac007bb4b80a4fd4ffffffff01e62e1900000000001976a914029f45cefe259733c9d860b70f7a8385596607bf88ac00000000"
            )
            .unwrap(),
        wtxid,
        size: 191,
        time: Time::from_consensus(1_513_622_125).unwrap(),
        vsize: 191,
        weight: 764,
        blockhash: "00000000000000000024fb37364cbf81fd49cc2d51c09c75c35433c3a1945d04"
            .parse()
            .unwrap(),
        blocktime: Time::from_consensus(1_513_622_125).unwrap(),
        confirmations: tx.confirmations,
        locktime: LockTime::ZERO,
        vin: vec![VinSpecific {
            txid: "e29059d4bae0a533daeb8ffbd07a072c9ae019b047f0249bb56626d250fbde3c"
                .parse()
                .unwrap(),
            sequence: Sequence(4_294_967_295),
            vout: 0,
            tx_in_witness: None,
            script_sig: ScriptSig {
                asm: "304402202015dfc5b5d9030f9538c1f6e0b99fe8dbf46260044e45ad2f883744292af09b0220066353c0d19f9734278ba7072fa8b3ba1c1c30bdd583721439b8ee375a098ad8[ALL] 03de2010f23c4eda698d373cfc8f7ecd576fbb4e40f67a8634ac007bb4b80a4fd4"
                    .into(),
                script: ScriptBuf::from_hex(
                    "47304402202015dfc5b5d9030f9538c1f6e0b99fe8dbf46260044e45ad2f883744292af09b0220066353c0d19f9734278ba7072fa8b3ba1c1c30bdd583721439b8ee375a098ad8012103de2010f23c4eda698d373cfc8f7ecd576fbb4e40f67a8634ac007bb4b80a4fd4"
                    )
                    .unwrap(),
            },
        }],
        vout: vec![VoutSpecific {
            value: Amount::from_sat(1_650_406),
            n: 0,
            script_pub_key: ScriptPubKey {
                address: "1Es9qximvz5W9TqtZfxx9cV7thmvDutWf"
                    .parse::<Address<NetworkUnchecked>>()
                    .unwrap()
                    .assume_checked(),
                asm: "OP_DUP OP_HASH160 029f45cefe259733c9d860b70f7a8385596607bf OP_EQUALVERIFY OP_CHECKSIG"
                    .into(),
                desc: Some("addr(1Es9qximvz5W9TqtZfxx9cV7thmvDutWf)#4h8xrdj5".into()),
                script: ScriptBuf::from_hex("76a914029f45cefe259733c9d860b70f7a8385596607bf88ac")
                    .unwrap(),
                r#type: ScriptPubKeyType::PubKeyHash,
            },
        }],
    };
    assert_eq!(tx, expected_tx);
}

#[allow(clippy::too_many_lines)]
#[ignore]
#[tokio::test]
async fn test_block_by_hash() {
    let block = blockbook()
        .block_by_hash(
            "000000000003ba27aa200b1cecaad478d2b00432346c3f1f3986da1afd33e506"
                .parse()
                .unwrap(),
        )
        .await
        .unwrap();
    let expected_block = Block {
        page: 1,
        total_pages: 1,
        items_on_page: 1000,
        hash: "000000000003ba27aa200b1cecaad478d2b00432346c3f1f3986da1afd33e506"
            .parse()
            .unwrap(),
        previous_block_hash: Some(
            "000000000002d01c1fccc21636b607dfd930d31d01c3a62104612a1719011250"
                .parse()
                .unwrap(),
        ),
        next_block_hash: Some(
            "00000000000080b66c911bd5ba14a74260057311eaeb1982802f7010f1a9f090"
                .parse()
                .unwrap(),
        ),
        height: Height::from_consensus(100_000).unwrap(),
        confirmations: block.confirmations,
        size: 957,
        time: Time::from_consensus(1_293_623_863).unwrap(),
        version: 1,
        merkle_root: "f3e94742aca4b5ef85488dc37c06c3282295ffec960994b2c0d5ac2a25a95766"
            .parse()
            .unwrap(),
        nonce: "274148111".into(),
        bits: "1b04864c".into(),
        difficulty: "14484.1623612254".into(),
        tx_count: 4,
        txs: vec![
            BlockTransaction {
                txid: "8c14f0db3df150123e6f3dbbf30f8b955a8249b62ac1d1ff16284aefa3d06d87"
                    .parse()
                    .unwrap(),
                vin: vec![BlockVin {
                    n: 0,
                    addresses: None,
                    is_address: false,
                    value: Amount::ZERO,
                }],
                vout: vec![BlockVout {
                    value: Amount::from_sat(5_000_000_000),
                    n: 0,
                    spent: Some(true),
                    addresses: vec![AddressBlockVout::Address(
                        "1HWqMzw1jfpXb3xyuUZ4uWXY4tqL2cW47J"
                            .parse::<Address<NetworkUnchecked>>()
                            .unwrap()
                            .assume_checked(),
                    )],
                    is_address: true,
                }],
                block_hash: "000000000003ba27aa200b1cecaad478d2b00432346c3f1f3986da1afd33e506"
                    .parse()
                    .unwrap(),
                block_height: Height::from_consensus(100_000).unwrap(),
                confirmations: block.confirmations,
                block_time: Time::from_consensus(1_293_623_863).unwrap(),
                value: Amount::from_sat(5_000_000_000),
                value_in: Amount::ZERO,
                fees: Amount::ZERO,
            },
            BlockTransaction {
                txid: "fff2525b8931402dd09222c50775608f75787bd2b87e56995a7bdd30f79702c4"
                    .parse()
                    .unwrap(),
                vin: vec![BlockVin {
                    n: 0,
                    addresses: Some(vec!["1BNwxHGaFbeUBitpjy2AsKpJ29Ybxntqvb"
                        .parse::<Address<NetworkUnchecked>>()
                        .unwrap()
                        .assume_checked()]),
                    is_address: true,
                    value: Amount::from_sat(5_000_000_000),
                }],
                vout: vec![
                    BlockVout {
                        value: Amount::from_sat(556_000_000),
                        n: 0,
                        spent: Some(true),
                        addresses: vec![AddressBlockVout::Address(
                            "1JqDybm2nWTENrHvMyafbSXXtTk5Uv5QAn"
                                .parse::<Address<NetworkUnchecked>>()
                                .unwrap()
                                .assume_checked(),
                        )],
                        is_address: true,
                    },
                    BlockVout {
                        value: Amount::from_sat(4_444_000_000),
                        n: 1,
                        spent: Some(true),
                        addresses: vec![AddressBlockVout::Address(
                            "1EYTGtG4LnFfiMvjJdsU7GMGCQvsRSjYhx"
                                .parse::<Address<NetworkUnchecked>>()
                                .unwrap()
                                .assume_checked(),
                        )],
                        is_address: true,
                    },
                ],
                block_hash: "000000000003ba27aa200b1cecaad478d2b00432346c3f1f3986da1afd33e506"
                    .parse()
                    .unwrap(),
                block_height: Height::from_consensus(100_000).unwrap(),
                confirmations: block.confirmations,
                block_time: Time::from_consensus(1_293_623_863).unwrap(),
                value: Amount::from_sat(5_000_000_000),
                value_in: Amount::from_sat(5_000_000_000),
                fees: Amount::ZERO,
            },
            BlockTransaction {
                txid: "6359f0868171b1d194cbee1af2f16ea598ae8fad666d9b012c8ed2b79a236ec4"
                    .parse()
                    .unwrap(),
                vin: vec![BlockVin {
                    n: 0,
                    addresses: Some(vec!["15vScfMHNrXN4QvWe54q5hwfVoYwG79CS1"
                        .parse::<Address<NetworkUnchecked>>()
                        .unwrap()
                        .assume_checked()]),
                    is_address: true,
                    value: Amount::from_sat(300_000_000),
                }],
                vout: vec![
                    BlockVout {
                        value: Amount::from_sat(1_000_000),
                        n: 0,
                        spent: Some(true),
                        addresses: vec![AddressBlockVout::Address(
                            "1H8ANdafjpqYntniT3Ddxh4xPBMCSz33pj"
                                .parse::<Address<NetworkUnchecked>>()
                                .unwrap()
                                .assume_checked(),
                        )],
                        is_address: true,
                    },
                    BlockVout {
                        value: Amount::from_sat(299_000_000),
                        n: 1,
                        spent: Some(true),
                        addresses: vec![AddressBlockVout::Address(
                            "1Am9UTGfdnxabvcywYG2hvzr6qK8T3oUZT"
                                .parse::<Address<NetworkUnchecked>>()
                                .unwrap()
                                .assume_checked(),
                        )],
                        is_address: true,
                    },
                ],
                block_hash: "000000000003ba27aa200b1cecaad478d2b00432346c3f1f3986da1afd33e506"
                    .parse()
                    .unwrap(),
                block_height: Height::from_consensus(100_000).unwrap(),
                confirmations: block.confirmations,
                block_time: Time::from_consensus(1_293_623_863).unwrap(),
                value: Amount::from_sat(300_000_000),
                value_in: Amount::from_sat(300_000_000),
                fees: Amount::ZERO,
            },
            BlockTransaction {
                txid: "e9a66845e05d5abc0ad04ec80f774a7e585c6e8db975962d069a522137b80c1d"
                    .parse()
                    .unwrap(),
                vin: vec![BlockVin {
                    n: 0,
                    addresses: Some(vec!["1JxDJCyWNakZ5kECKdCU9Zka6mh34mZ7B2"
                        .parse::<Address<NetworkUnchecked>>()
                        .unwrap()
                        .assume_checked()]),
                    is_address: true,
                    value: Amount::from_sat(1_000_000),
                }],
                vout: vec![BlockVout {
                    value: Amount::from_sat(1_000_000),
                    n: 0,
                    spent: Some(true),
                    addresses: vec![AddressBlockVout::Address(
                        "16FuTPaeRSPVxxCnwQmdyx2PQWxX6HWzhQ"
                            .parse::<Address<NetworkUnchecked>>()
                            .unwrap()
                            .assume_checked(),
                    )],
                    is_address: true,
                }],
                block_hash: "000000000003ba27aa200b1cecaad478d2b00432346c3f1f3986da1afd33e506"
                    .parse()
                    .unwrap(),
                block_height: Height::from_consensus(100_000).unwrap(),
                confirmations: block.confirmations,
                block_time: Time::from_consensus(1_293_623_863).unwrap(),
                value: Amount::from_sat(1_000_000),
                value_in: Amount::from_sat(1_000_000),
                fees: Amount::ZERO,
            },
        ],
    };
    assert_eq!(block, expected_block);
}

#[ignore]
#[tokio::test]
async fn test_block_by_height_with_opreturn_output() {
    let block = blockbook()
        .block_by_height(Height::from_consensus(500_044).unwrap())
        .await
        .unwrap();
    let expected_block = Block {
        page: 1,
        total_pages: 1,
        items_on_page: 1000,
        hash: "0000000000000000001f9ba01120351182680ceba085ffabeaa532cda35f2cc7"
            .parse()
            .unwrap(),
        previous_block_hash: Some(
            "00000000000000000075807003ae64990ee53422788451b795839f70a177695d"
                .parse()
                .unwrap(),
        ),
        next_block_hash: Some(
            "0000000000000000000401882bfcb7e7f14a14e21b827e5fb5981d07a5cea0f2"
                .parse()
                .unwrap(),
        ),
        height: Height::from_consensus(500_044).unwrap(),
        confirmations: block.confirmations,
        size: 285,
        time: Time::from_consensus(1_513_638_772).unwrap(),
        version: 536_870_912,
        merkle_root: "db5f956e4f48e79895021a9f7e64035fd03680e96253aafd438118485bfe49cb"
            .parse()
            .unwrap(),
        nonce: "437791427".into(),
        bits: "18009645".into(),
        difficulty: "1873105475221.611".into(),
        tx_count: 1,
        txs: vec![BlockTransaction {
            txid: "db5f956e4f48e79895021a9f7e64035fd03680e96253aafd438118485bfe49cb"
                .parse()
                .unwrap(),
            vin: vec![BlockVin {
                n: 0,
                addresses: None,
                is_address: false,
                value: Amount::ZERO,
            }],
            vout: vec![
                BlockVout {
                    value: Amount::from_sat(1_250_000_000),
                    n: 0,
                    spent: Some(true),
                    addresses: vec![AddressBlockVout::Address(
                        "1NS4gbx1G2D5rc9PnvVsPys12nKxGiQg72"
                        .parse::<Address<NetworkUnchecked>>()
                        .unwrap()
                        .assume_checked(),
                    )],
                    is_address: true,
                },
                BlockVout {
                    value: Amount::ZERO,
                    n: 1,
                    spent: block.txs.get(0).unwrap().vout.get(1).unwrap().spent,
                    addresses: vec![AddressBlockVout::OpReturn(OpReturn(
                        "OP_RETURN aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf9".into())
                )],
                    is_address: false,
                },
            ],
            block_hash: "0000000000000000001f9ba01120351182680ceba085ffabeaa532cda35f2cc7"
                .parse()
                .unwrap(),
            block_height: Height::from_consensus(500_044).unwrap(),
            confirmations: block.confirmations,
            block_time: Time::from_consensus(1_513_638_772).unwrap(),
            value: Amount::from_sat(1_250_000_000),
            value_in: Amount::ZERO,
            fees: Amount::ZERO,
        }],
    };
    assert_eq!(block, expected_block);
}

#[ignore]
#[tokio::test]
async fn test_tickers_list() {
    let tickers_list = blockbook()
        .tickers_list(Time::from_consensus(1_674_821_349).unwrap())
        .await
        .unwrap();
    let expected_tickers_list = TickersList {
        timestamp: Time::from_consensus(1_674_864_000).unwrap(),
        available_currencies: vec![
            Currency::Aed,
            Currency::Ars,
            Currency::Aud,
            Currency::Bdt,
            Currency::Bhd,
            Currency::Bmd,
            Currency::Brl,
            Currency::Btc,
            Currency::Cad,
            Currency::Chf,
            Currency::Clp,
            Currency::Cny,
            Currency::Czk,
            Currency::Dkk,
            Currency::Eth,
            Currency::Eur,
            Currency::Gbp,
            Currency::Hkd,
            Currency::Huf,
            Currency::Idr,
            Currency::Ils,
            Currency::Inr,
            Currency::Jpy,
            Currency::Krw,
            Currency::Kwd,
            Currency::Lkr,
            Currency::Mmk,
            Currency::Mxn,
            Currency::Myr,
            Currency::Ngn,
            Currency::Nok,
            Currency::Nzd,
            Currency::Php,
            Currency::Pkr,
            Currency::Pln,
            Currency::Rub,
            Currency::Sar,
            Currency::Sek,
            Currency::Sgd,
            Currency::Thb,
            Currency::Try,
            Currency::Twd,
            Currency::Uah,
            Currency::Usd,
            Currency::Vef,
            Currency::Vnd,
            Currency::Zar,
        ],
    };
    assert_eq!(tickers_list, expected_tickers_list);
}

#[ignore]
#[tokio::test]
async fn test_current_tickers_list() {
    blockbook()
        .tickers_list(
            Time::from_consensus(
                std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .try_into()
                    .unwrap(),
            )
            .unwrap(),
        )
        .await
        .unwrap();
}

#[ignore]
#[tokio::test]
async fn test_tickers() {
    let tickers = blockbook()
        .tickers(Some(Time::from_consensus(1_674_821_349).unwrap()))
        .await
        .unwrap();
    let expected_tickers = Ticker {
        timestamp: Time::from_consensus(1_674_864_000).unwrap(),
        rates: std::collections::HashMap::from([
            (Currency::Aed, 84_783.336),
            (Currency::Ars, 4_282_867.5),
            (Currency::Aud, 32_274.684),
            (Currency::Bdt, 2_447_401.8),
            (Currency::Bhd, 8_703.507),
            (Currency::Bmd, 23_082.857),
            (Currency::Brl, 117_937.25),
            (Currency::Btc, 1.0),
            (Currency::Cad, 30_791.379),
            (Currency::Chf, 21_293.938),
            (Currency::Clp, 18_614_248.0),
            (Currency::Cny, 156_582.56),
            (Currency::Czk, 505_740.8),
            (Currency::Dkk, 157_962.92),
            (Currency::Eth, 14.43425),
            (Currency::Eur, 21_232.443),
            (Currency::Gbp, 18_643.771),
            (Currency::Hkd, 180_742.25),
            (Currency::Huf, 8_293_902.0),
            (Currency::Idr, 345_612_700.0),
            (Currency::Ils, 79_411.266),
            (Currency::Inr, 1_881_595.6),
            (Currency::Jpy, 2_998_118.5),
            (Currency::Krw, 28_393_070.0),
            (Currency::Kwd, 7_044.703_6),
            (Currency::Lkr, 8_404_290.0),
            (Currency::Mmk, 48_486_224.0),
            (Currency::Mxn, 433_456.84),
            (Currency::Myr, 97_963.65),
            (Currency::Ngn, 10_624_578.0),
            (Currency::Nok, 228_294.1),
            (Currency::Nzd, 35_566.81),
            (Currency::Php, 1_260_220.1),
            (Currency::Pkr, 5_783_717.5),
            (Currency::Pln, 99_952.24),
            (Currency::Rub, 1_609_952.6),
            (Currency::Sar, 86_651.664),
            (Currency::Sek, 237_953.11),
            (Currency::Sgd, 30_321.643),
            (Currency::Thb, 756_436.75),
            (Currency::Try, 434_174.72),
            (Currency::Twd, 698_637.3),
            (Currency::Uah, 848_393.44),
            (Currency::Usd, 23_082.857),
            (Currency::Vef, 2_311.286_6),
            (Currency::Vnd, 541_523_840.0),
            (Currency::Zar, 397_099.03),
        ]),
    };
    assert_eq!(tickers, expected_tickers);
}

#[ignore]
#[tokio::test]
async fn test_current_tickers() {
    blockbook().tickers(None).await.unwrap();
}

#[ignore]
#[tokio::test]
async fn test_ticker() {
    let ticker = blockbook()
        .ticker(
            Currency::Idr,
            Some(Time::from_consensus(1_674_692_106).unwrap()),
        )
        .await
        .unwrap();
    let expected_ticker = Ticker {
        timestamp: Time::from_consensus(1_674_777_600).unwrap(),
        rates: std::collections::HashMap::from([(Currency::Idr, 344_337_380.0)]),
    };
    assert_eq!(ticker, expected_ticker);
}

#[ignore]
#[tokio::test]
async fn test_info_ws() {
    let info = blockbook_ws().await.info().await.unwrap();
    let expected_info = Info {
        name: "Bitcoin".into(),
        shortcut: "BTC".into(),
        decimals: 8,
        version: semver::Version::new(0, 4, 0),
        best_height: info.best_height,
        best_hash: info.best_hash,
        block_0_hash: "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f"
            .parse()
            .unwrap(),
        testnet: false,
        backend_version: Version {
            version: "240001".into(),
            subversion: "/Satoshi:24.0.1/".into(),
        },
    };
    assert_eq!(info, expected_info);
}

#[ignore]
#[tokio::test]
async fn test_block_hash_ws() {
    assert_eq!(
        blockbook_ws()
            .await
            .block_hash(Height::from_consensus(500_044).unwrap())
            .await
            .unwrap()
            .to_string(),
        "0000000000000000001f9ba01120351182680ceba085ffabeaa532cda35f2cc7"
    );
}

#[ignore]
#[tokio::test]
async fn test_current_fiat_rates() {
    let mut client = blockbook_ws().await;
    let rates = client
        .current_fiat_rates(vec![Currency::Btc, Currency::Chf])
        .await
        .unwrap()
        .rates;
    assert!((rates.get(&Currency::Btc).unwrap() - 1.0).abs() < f64::EPSILON);
    assert!(rates.get(&Currency::Chf).unwrap() > &0.0);
}

#[ignore]
#[tokio::test]
async fn test_available_currencies() {
    let mut client = blockbook_ws().await;
    let tickers = client
        .available_currencies(Time::from_consensus(1_682_415_368).unwrap())
        .await
        .unwrap()
        .available_currencies;
    assert!(tickers.contains(&Currency::Btc));
    assert!(tickers.contains(&Currency::Chf));
}

#[ignore]
#[tokio::test]
async fn test_fiat_rates_for_timestamps() {
    let mut client = blockbook_ws().await;
    let tickers = client
        .fiat_rates_for_timestamps(vec![Time::from_consensus(1_575_288_000).unwrap()], None)
        .await
        .unwrap();
    assert_eq!(tickers.len(), 1);
    let ticker = tickers.get(0).unwrap();
    assert_eq!(ticker.timestamp.to_consensus_u32(), 1_575_331_200);
    assert!(ticker.rates.contains_key(&Currency::Chf));
    assert!(ticker.rates.contains_key(&Currency::Cad));
    let tickers = client
        .fiat_rates_for_timestamps(
            vec![
                Time::from_consensus(1_575_288_000).unwrap(),
                Time::from_consensus(1_675_288_000).unwrap(),
            ],
            Some(vec![Currency::Chf, Currency::Usd]),
        )
        .await
        .unwrap();
    assert_eq!(tickers.len(), 2);
    assert_eq!(
        tickers.get(0).unwrap().timestamp.to_consensus_u32(),
        1_575_331_200
    );
    assert_eq!(
        tickers.get(1).unwrap().timestamp.to_consensus_u32(),
        1_675_296_000
    );
    assert_eq!(tickers.get(0).unwrap().rates.len(), 2);
    assert_eq!(tickers.get(1).unwrap().rates.len(), 2);
}

#[ignore]
#[tokio::test]
async fn test_estimate_fee() {
    let mut client = blockbook_ws().await;
    let fees = client
        .estimate_fee(vec![1, 2, 5, 10, 100, 1000])
        .await
        .unwrap();
    let mut fees_cloned = fees.clone();
    // should already be reverse-sorted
    fees_cloned.sort_by(|a, b| b.cmp(a));
    assert_eq!(fees, fees_cloned);
    fees.iter().for_each(|f| assert!(f.to_sat() > 0));
}

#[ignore]
#[tokio::test]
async fn test_estimate_tx_fee() {
    let mut client = blockbook_ws().await;
    let fees = client
        .estimate_tx_fee(vec![1, 2, 5, 10, 100, 1000], 600)
        .await
        .unwrap();
    let mut fees_cloned = fees.clone();
    // should already be reverse-sorted
    fees_cloned.sort_by(|a, b| b.cmp(a));
    assert_eq!(fees, fees_cloned);
    fees.iter().for_each(|f| assert!(f.to_sat() > 0));
}

#[ignore]
#[tokio::test]
async fn test_utxos_from_address_ws() {
    let mut client = blockbook_ws().await;
    let utxos = client
        .utxos_from_address(
            "bc1q5au2nmza9pmplnvgzyd4ky7egu2wya56qa024u"
                .parse::<Address<NetworkUnchecked>>()
                .unwrap()
                .assume_checked(),
        )
        .await
        .unwrap();
    assert_eq!(utxos.len(), 1);
    let expected_utxo = Utxo {
        txid: "d60691ecdf678eb3f88ebfaf315d3907d0be62ccd40fd1f027938249966f268d"
            .parse()
            .unwrap(),
        vout: 1,
        value: Amount::from_sat(821),
        height: Some(Height::from_consensus(784_027).unwrap()),
        confirmations: utxos.get(0).unwrap().confirmations,
        locktime: None,
        address: None,
        path: None,
        coinbase: None,
    };
    assert_eq!(utxos.get(0).unwrap(), &expected_utxo);
}

#[ignore]
#[tokio::test]
async fn test_balance_history() {
    let client = blockbook();
    let mut ws_client = blockbook_ws().await;
    let address: Address = "399WojVNByUJZSuJDAKYJ8EQmbkDUgHuT6"
        .parse::<Address<NetworkUnchecked>>()
        .unwrap()
        .assume_checked();
    let from = Time::from_consensus(1_676_000_000).unwrap();
    let to = Time::from_consensus(1_682_000_000).unwrap();
    let group_by = 1_000_000;
    let history_1 = ws_client
        .balance_history(
            address.clone(),
            Some(from),
            Some(to),
            Some(vec![Currency::Chf]),
            Some(group_by),
        )
        .await
        .unwrap();
    assert_eq!(
        history_1,
        client
            .balance_history(
                &address,
                Some(from),
                Some(to),
                Some(Currency::Chf),
                Some(group_by)
            )
            .await
            .unwrap()
    );
    assert_eq!(history_1.len(), 6);
    let tx_count_1: u32 = history_1.iter().map(|entry| entry.txs).sum();
    let received_count_1: Amount = history_1.iter().map(|entry| entry.received).sum();

    let group_by = 2_000_000;
    let history_2 = ws_client
        .balance_history(
            address.clone(),
            Some(from),
            Some(to),
            Some(vec![Currency::Chf]),
            Some(group_by),
        )
        .await
        .unwrap();
    assert_eq!(
        history_2,
        client
            .balance_history(
                &address,
                Some(from),
                Some(to),
                Some(Currency::Chf),
                Some(group_by)
            )
            .await
            .unwrap()
    );
    assert_eq!(history_2.len(), 3);
    let tx_count_2: u32 = history_2.iter().map(|entry| entry.txs).sum();
    let received_count_2 = history_2.iter().map(|entry| entry.received).sum();

    assert_eq!(tx_count_1, tx_count_2);
    assert_eq!(received_count_1, received_count_2);
}

fn addr_1() -> Address {
    "bc1qsej2fzpejkar82t8nyc2dhkvk54kn905vpvzpw"
        .parse::<Address<NetworkUnchecked>>()
        .unwrap()
        .assume_checked()
}

#[ignore]
#[tokio::test]
async fn test_address_info() {
    let address_info = blockbook().address_info(&addr_1()).await.unwrap();
    assert_eq!(&address_info.basic.address, &addr_1());
    assert_eq!(
        address_info.txids.last().unwrap(),
        &Txid::from_str("98f08111f08baba3d33af28c74facc223a07d868c0568258980119761dea441d")
            .unwrap()
    );
}

#[ignore]
#[tokio::test]
async fn test_address_info_specific_no_args() {
    let address_info = blockbook()
        .address_info_specific(&addr_1(), None, None, None, None, None)
        .await
        .unwrap();
    assert_eq!(&address_info.basic.address, &addr_1());
    assert_eq!(
        address_info.txids.last().unwrap(),
        &Txid::from_str("98f08111f08baba3d33af28c74facc223a07d868c0568258980119761dea441d")
            .unwrap()
    );
    let mut websocket_info = blockbook_ws()
        .await
        .address_info_txids(addr_1(), None, None, None, None, None)
        .await
        .unwrap();
    websocket_info.paging.items_on_page = address_info.paging.items_on_page;
    assert_eq!(address_info, websocket_info);
}

fn counterparty_burner_addr() -> Address {
    "1CounterpartyXXXXXXXXXXXXXXXUWLpVr"
        .parse::<Address<NetworkUnchecked>>()
        .unwrap()
        .assume_checked()
}

#[ignore]
#[tokio::test]
async fn test_address_info_specific_page() {
    let address = counterparty_burner_addr();
    let number_of_txs = blockbook()
        .address_info_specific_basic(&address, None)
        .await
        .unwrap()
        .txs;
    let address_info = blockbook()
        .address_info_specific(
            &address,
            Some(&std::num::NonZeroU32::new(number_of_txs).unwrap()),
            Some(&std::num::NonZeroU16::new(1).unwrap()),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(&address_info.basic.address, &address);
    assert_eq!(
        address_info.txids.get(0).unwrap(),
        &Txid::from_str("685623401c3f5e9d2eaaf0657a50454e56a270ee7630d409e98d3bc257560098")
            .unwrap(),
    );
    let websocket_info = blockbook_ws()
        .await
        .address_info_txids(
            address,
            Some(std::num::NonZeroU32::new(number_of_txs).unwrap()),
            Some(std::num::NonZeroU16::new(1).unwrap()),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(address_info, websocket_info);
}

fn addr_2() -> Address {
    "3Kzh9qAqVWQhEsfQz7zEQL1EuSx5tyNLNS"
        .parse::<Address<NetworkUnchecked>>()
        .unwrap()
        .assume_checked()
}

#[ignore]
#[tokio::test]
async fn test_address_info_specific_blocks_basic() {
    let address_info = blockbook()
        .address_info_specific_basic(&addr_2(), Some(&Currency::Usd))
        .await
        .unwrap();
    assert_eq!(&address_info.address, &addr_2());
    assert_eq!(
        address_info,
        blockbook_ws()
            .await
            .address_info_basic(addr_2(), Some(Currency::Usd))
            .await
            .unwrap()
    );
}

#[ignore]
#[tokio::test]
async fn test_address_info_specific_blocks() {
    let address_info = blockbook()
        .address_info_specific(
            &addr_2(),
            None,
            None,
            Some(&Height::from_consensus(500_000).unwrap()),
            Some(&Height::from_consensus(503_000).unwrap()),
            None,
        )
        .await
        .unwrap();
    let expected_address_info = AddressInfo {
        paging: AddressInfoPaging {
            page: 1,
            total_pages: address_info.paging.total_pages,
            items_on_page: 1000,
        },
        basic: AddressInfoBasic {
            address: addr_2(),
            balance: address_info.basic.balance,
            total_received: address_info.basic.total_received,
            total_sent: address_info.basic.total_sent,
            unconfirmed_balance: address_info.basic.unconfirmed_balance,
            unconfirmed_txs: address_info.basic.unconfirmed_txs,
            txs: address_info.basic.txs,
            secondary_value: None,
        },
        txids: vec![
            "ae1484c0cecf39700bb1697793bec24fbb1980207eeb1374eb293a5c403ac8c3"
                .parse()
                .unwrap(),
            "a4b4b879af01563cccadca66d36a0f47afcf78f263bed0966df8abf0a2699f3d"
                .parse()
                .unwrap(),
            "e09390893277b6957cf93ad9ec4b72c6c140aceaa8e62874151ebfca403a76e1"
                .parse()
                .unwrap(),
            "67a6147be5216a0b77e87002e9911f62e2b3dcfa44ce15e8c28e39d77860c59e"
                .parse()
                .unwrap(),
            "2c3ca46df14114490e5d22ddcbcf08a730854e7554a54094c0fb4d7b7a576ed7"
                .parse()
                .unwrap(),
        ],
    };
    assert_eq!(address_info, expected_address_info);
    let mut websocket_info = blockbook_ws()
        .await
        .address_info_txids(
            addr_2(),
            None,
            None,
            Some(Height::from_consensus(500_000).unwrap()),
            Some(Height::from_consensus(503_000).unwrap()),
            None,
        )
        .await
        .unwrap();
    websocket_info.paging.items_on_page = address_info.paging.items_on_page;
    assert_eq!(address_info, websocket_info);
}

#[allow(clippy::too_many_lines)]
#[ignore]
#[tokio::test]
async fn test_address_info_specific_blocks_details() {
    let address_info = blockbook()
        .address_info_specific_detailed(
            &addr_2(),
            None,
            None,
            Some(&Height::from_consensus(501_000).unwrap()),
            Some(&Height::from_consensus(502_000).unwrap()),
            &TxDetail::Full,
            Some(&Currency::Zar),
        )
        .await
        .unwrap();
    let expected_address_info = AddressInfoDetailed {
        paging: AddressInfoPaging {
            page: 1,
            total_pages: address_info.paging.total_pages,
            items_on_page: 1000,
        },
        basic: AddressInfoBasic {
            address: addr_2(),
            balance: address_info.basic.balance,
            total_received: address_info.basic.total_received,
            total_sent: address_info.basic.total_sent,
            unconfirmed_balance: address_info.basic.unconfirmed_balance,
            unconfirmed_txs: address_info.basic.unconfirmed_txs,
            txs: address_info.basic.txs,
            secondary_value: address_info.basic.secondary_value,
        },
        transactions: vec![Tx::Ordinary(Transaction {
            txid: "67a6147be5216a0b77e87002e9911f62e2b3dcfa44ce15e8c28e39d77860c59e"
                .parse()
                .unwrap(),
            version: 1,
            lock_time: None,
            vin: vec![Vin {
                txid: "a563f78cac895c1abf411eb93000f751cf20c94e9f32360e643841e37080f906"
                    .parse()
                    .unwrap(),
                vout: Some(1),
                sequence: Some(Sequence(4_294_967_295)),
                is_address: true,
                value: Amount::from_sat(3_084_293),
                n: 0,
                addresses: vec!["1LyqvGRjLoznNX2RbytTvuyswDpDVqoYt7"
                    .parse::<Address<NetworkUnchecked>>()
                    .unwrap()
                    .assume_checked()],
            }],
            vout: vec![
                Vout {
                    value: Amount::from_sat(11_700),
                    n: 0,
                    spent: Some(true),
                    script: ScriptBuf::from_hex("a914c8ca150ee82589d47f69b8dcd7cad684d88283f187")
                        .unwrap(),
                    addresses: vec![addr_2()],
                    is_address: true,
                },
                Vout {
                    value: Amount::from_sat(3_049_993),
                    n: 1,
                    spent: Some(true),
                    script: ScriptBuf::from_hex("76a9147d55684397c290fbc638bdc52528350088b8837488ac")
                        .unwrap(),
                    addresses: vec!["1CRhnBV2q8ToQcaKMBBkeooJdNX9ohSWDc"
                        .parse::<Address<NetworkUnchecked>>()
                        .unwrap()
                        .assume_checked()],
                    is_address: true,
                },
            ],
            size: 223,
            vsize: 223,
            block_hash: Some("0000000000000000001617fb8817ecd53e1093247bcf813b2eae793033af0c0a"
                .parse()
                .unwrap()),
            block_height: Some(Height::from_consensus(501_498).unwrap()),
            confirmations: if let Tx::Ordinary(tx) = &address_info.transactions.get(0).unwrap() {
                tx.confirmations
            } else {
                panic!()
            },
            block_time: Time::from_consensus(1_514_513_020).unwrap(),
            value: Amount::from_sat(3_061_693),
            value_in: Amount::from_sat(3_084_293),
            fees: Amount::from_sat(22_600),
            script: ScriptBuf::from_hex(
                "010000000106f98070e34138640e36329f4ec920cf51f70030b91e41bf1a5c89ac8cf763a5010000006a47304402200a54b9076c0fd91c3bcaa5c55a0721d18893b6aa204c87198d072555aff3bf2e02206aa296427da8eb404203044e9e33d5f69dbbdbc43be95dd4ac5c675b8c341c7301210241d3f009960b9695c8b7c546128aa4d01daf57c4ff562f6d1f30c2a85119af1cffffffff02b42d00000000000017a914c8ca150ee82589d47f69b8dcd7cad684d88283f187098a2e00000000001976a9147d55684397c290fbc638bdc52528350088b8837488ac00000000"
                )
                .unwrap(),
        })],
    };
    assert_eq!(&address_info, &expected_address_info);
    let mut websocket_info = blockbook_ws()
        .await
        .address_info_txs(
            addr_2(),
            None,
            None,
            Some(Height::from_consensus(501_000).unwrap()),
            Some(Height::from_consensus(502_000).unwrap()),
            Some(Currency::Zar),
        )
        .await
        .unwrap();
    websocket_info.paging.items_on_page = address_info.paging.items_on_page;
    assert_eq!(websocket_info, expected_address_info);
}

#[ignore]
#[tokio::test]
async fn test_address_info_specific_blocks_details_light() {
    let address_info = blockbook()
        .address_info_specific_detailed(
            &addr_2(),
            None,
            None,
            Some(&Height::from_consensus(501_000).unwrap()),
            Some(&Height::from_consensus(502_000).unwrap()),
            &TxDetail::Light,
            None,
        )
        .await
        .unwrap();
    let expected_address_info = AddressInfoDetailed {
        paging: AddressInfoPaging {
            page: 1,
            total_pages: address_info.paging.total_pages,
            items_on_page: 1000,
        },
        basic: AddressInfoBasic {
            address: addr_2(),
            balance: address_info.basic.balance,
            total_received: address_info.basic.total_received,
            total_sent: address_info.basic.total_sent,
            unconfirmed_balance: address_info.basic.unconfirmed_balance,
            unconfirmed_txs: address_info.basic.unconfirmed_txs,
            txs: address_info.basic.txs,
            secondary_value: None,
        },
        transactions: vec![Tx::Light(BlockTransaction {
            txid: "67a6147be5216a0b77e87002e9911f62e2b3dcfa44ce15e8c28e39d77860c59e"
                .parse()
                .unwrap(),
            vin: vec![BlockVin {
                is_address: true,
                value: Amount::from_sat(3_084_293),
                n: 0,
                addresses: Some(vec!["1LyqvGRjLoznNX2RbytTvuyswDpDVqoYt7"
                    .parse::<Address<NetworkUnchecked>>()
                    .unwrap()
                    .assume_checked()]),
            }],
            vout: vec![
                BlockVout {
                    value: Amount::from_sat(11_700),
                    n: 0,
                    spent: Some(true),
                    addresses: vec![AddressBlockVout::Address(addr_2())],
                    is_address: true,
                },
                BlockVout {
                    value: Amount::from_sat(3_049_993),
                    n: 1,
                    spent: Some(true),
                    addresses: vec![AddressBlockVout::Address(
                        "1CRhnBV2q8ToQcaKMBBkeooJdNX9ohSWDc"
                            .parse::<Address<NetworkUnchecked>>()
                            .unwrap()
                            .assume_checked(),
                    )],
                    is_address: true,
                },
            ],
            block_hash: "0000000000000000001617fb8817ecd53e1093247bcf813b2eae793033af0c0a"
                .parse()
                .unwrap(),
            block_height: Height::from_consensus(501_498).unwrap(),
            confirmations: if let Tx::Light(tx) = &address_info.transactions.get(0).unwrap() {
                tx.confirmations
            } else {
                panic!()
            },
            block_time: Time::from_consensus(1_514_513_020).unwrap(),
            value: Amount::from_sat(3_061_693),
            value_in: Amount::from_sat(3_084_293),
            fees: Amount::from_sat(22_600),
        })],
    };
    assert_eq!(&address_info, &expected_address_info);
}

#[ignore]
#[tokio::test]
async fn test_address_info_correct_variant_full() {
    let address_info_full = blockbook()
        .address_info_specific_detailed(
            &"bc1qhjhn2gm6mv4k99942ud4spe54483drh330faax"
                .parse::<Address<NetworkUnchecked>>()
                .unwrap()
                .assume_checked(),
            None,
            None,
            None,
            None,
            &TxDetail::Full,
            None,
        )
        .await
        .unwrap();
    assert!(matches!(
        address_info_full.transactions.get(0).unwrap(),
        Tx::Ordinary(..)
    ));
}

#[ignore]
#[tokio::test]
async fn test_address_info_correct_variant_light() {
    let address_info_light = blockbook()
        .address_info_specific_detailed(
            &"bc1qhjhn2gm6mv4k99942ud4spe54483drh330faax"
                .parse::<Address<NetworkUnchecked>>()
                .unwrap()
                .assume_checked(),
            None,
            None,
            None,
            None,
            &TxDetail::Light,
            None,
        )
        .await
        .unwrap();
    assert!(matches!(
        address_info_light.transactions.get(0).unwrap(),
        Tx::Light(..)
    ));
}

fn xpub_addr() -> Address {
    "bc1q5au2nmza9pmplnvgzyd4ky7egu2wya56qa024u"
        .parse::<Address<NetworkUnchecked>>()
        .unwrap()
        .assume_checked()
}

#[ignore]
#[tokio::test]
async fn test_address_info_block_and_pagesize_filter_combination() {
    let address_info = blockbook()
        .address_info_specific(
            &counterparty_burner_addr(),
            None,
            Some(&std::num::NonZeroU16::new(1).unwrap()),
            Some(&Height::from_consensus(600_000).unwrap()),
            Some(&Height::from_consensus(700_000).unwrap()),
            None,
        )
        .await
        .unwrap();
    assert_eq!(address_info.paging.total_pages, None);
    let mut websocket_info = blockbook_ws()
        .await
        .address_info_txids(
            counterparty_burner_addr(),
            None,
            Some(std::num::NonZeroU16::new(1).unwrap()),
            Some(Height::from_consensus(600_000).unwrap()),
            Some(Height::from_consensus(700_000).unwrap()),
            None,
        )
        .await
        .unwrap();
    websocket_info.paging.items_on_page = address_info.paging.items_on_page;
    assert_eq!(address_info, websocket_info);

    let address_info = blockbook()
        .address_info_specific(&xpub_addr(), None, None, None, None, None)
        .await
        .unwrap();
    assert_eq!(address_info.paging.total_pages, Some(1));
    let mut websocket_info = blockbook_ws()
        .await
        .address_info_txids(xpub_addr(), None, None, None, None, None)
        .await
        .unwrap();
    websocket_info.paging.items_on_page = address_info.paging.items_on_page;
    assert_eq!(address_info, websocket_info);
}

#[ignore]
#[tokio::test]
async fn test_utxos_from_address() {
    let utxos = blockbook()
        .utxos_from_address(counterparty_burner_addr(), false)
        .await
        .unwrap();
    let last_utxo = Utxo {
        txid: "685623401c3f5e9d2eaaf0657a50454e56a270ee7630d409e98d3bc257560098"
            .parse()
            .unwrap(),
        vout: 0,
        value: Amount::from_sat(50_000),
        height: Some(Height::from_consensus(278_319).unwrap()),
        confirmations: utxos.last().unwrap().confirmations,
        locktime: None,
        coinbase: None,
        address: None,
        path: None,
    };
    assert_eq!(utxos.last().unwrap(), &last_utxo);
}

fn xpub() -> &'static str {
    "zpub6qd36EtRVbyDyJToANn1vnuvhGenvepKnjeUzBXDk7JE4JYBxGUAPbjh22QqZ7JkRGuAtpgBfgKP1iT9GzgQxP1TgKPEBoN3e3vN3WtY2Su"
}

#[ignore]
#[tokio::test]
async fn test_utxos_from_xpub() {
    let utxos = blockbook().utxos_from_xpub(xpub(), true).await.unwrap();
    let expected_utxo = Utxo {
        txid: "d60691ecdf678eb3f88ebfaf315d3907d0be62ccd40fd1f027938249966f268d"
            .parse()
            .unwrap(),
        vout: 1,
        value: Amount::from_sat(821),
        height: Some(Height::from_consensus(784_027).unwrap()),
        confirmations: utxos.get(0).unwrap().confirmations,
        locktime: None,
        address: Some(xpub_addr()),
        path: Some("m/84'/0'/0'/0/0".parse().unwrap()),
        coinbase: None,
    };
    assert_eq!(utxos.get(0).unwrap(), &expected_utxo);
}

#[ignore]
#[tokio::test]
async fn test_xpub_info_basic_no_tokens() {
    let xpub_info = blockbook()
        .xpub_info_basic(xpub(), false, None, Some(&Currency::Btc))
        .await
        .unwrap();
    let expected_xpub_info = XPubInfoBasic {
        address: xpub().to_string(),
        balance: Amount::from_sat(821),
        total_received: Amount::from_sat(821),
        total_sent: Amount::from_sat(0),
        unconfirmed_balance: Amount::from_sat(0),
        unconfirmed_txs: 0,
        txs: 1,
        secondary_value: Some(821f64 / 10f64.powf(8.0)),
        used_tokens: 1,
        tokens: None,
    };
    assert_eq!(xpub_info, expected_xpub_info);
}

fn token() -> Token {
    Token {
        r#type: "XPUBAddress".to_string(),
        address: "bc1q5au2nmza9pmplnvgzyd4ky7egu2wya56qa024u"
            .parse::<Address<NetworkUnchecked>>()
            .unwrap()
            .assume_checked(),
        path: "m/84'/0'/0'/0/0".parse().unwrap(),
        transfers: 1,
        decimals: 8,
        balance: Amount::from_sat(821),
        total_received: Amount::from_sat(821),
        total_sent: Amount::from_sat(0),
    }
}

#[ignore]
#[tokio::test]
async fn test_xpub_info_txids() {
    let xpub_info = blockbook()
        .xpub_info(xpub(), None, None, None, None, false, None, None)
        .await
        .unwrap();
    let expected_xpub_info = XPubInfo {
        paging: AddressInfoPaging {
            page: 1,
            total_pages: Some(1),
            items_on_page: 1000,
        },
        basic: XPubInfoBasic {
            address: xpub().to_string(),
            balance: Amount::from_sat(821),
            total_received: Amount::from_sat(821),
            total_sent: Amount::from_sat(0),
            unconfirmed_balance: Amount::from_sat(0),
            unconfirmed_txs: 0,
            txs: 1,
            secondary_value: None,
            used_tokens: 1,
            tokens: Some(vec![token()]),
        },
        txids: Some(vec![
            "d60691ecdf678eb3f88ebfaf315d3907d0be62ccd40fd1f027938249966f268d"
                .parse()
                .unwrap(),
        ]),
        transactions: None,
    };
    assert_eq!(xpub_info, expected_xpub_info);
}

#[ignore]
#[tokio::test]
async fn test_xpub_info_specific() {
    let xpub_info = blockbook()
        .xpub_info(
            xpub(),
            Some(&std::num::NonZeroU32::new(1).unwrap()),
            Some(&std::num::NonZeroU16::new(1).unwrap()),
            Some(&Height::from_consensus(784_026).unwrap()),
            Some(&Height::from_consensus(784_028).unwrap()),
            false,
            Some(&AddressFilter::Used),
            Some(&Currency::Btc),
        )
        .await
        .unwrap();
    let expected_xpub_info = XPubInfo {
        paging: AddressInfoPaging {
            page: 1,
            total_pages: None,
            items_on_page: 1,
        },
        basic: XPubInfoBasic {
            address: xpub().to_string(),
            balance: Amount::from_sat(821),
            total_received: Amount::from_sat(821),
            total_sent: Amount::from_sat(0),
            unconfirmed_balance: Amount::from_sat(0),
            unconfirmed_txs: 0,
            txs: 1,
            secondary_value: Some(0.000_008_21),
            used_tokens: 1,
            tokens: Some(vec![token()]),
        },
        txids: Some(vec![
            "d60691ecdf678eb3f88ebfaf315d3907d0be62ccd40fd1f027938249966f268d"
                .parse()
                .unwrap(),
        ]),
        transactions: None,
    };
    assert_eq!(xpub_info, expected_xpub_info);
}

#[ignore]
#[tokio::test]
async fn test_xpub_info_empty() {
    let xpub_info = blockbook()
        .xpub_info(
            xpub(),
            None,
            None,
            Some(&Height::from_consensus(784_000).unwrap()),
            Some(&Height::from_consensus(784_002).unwrap()),
            false,
            None,
            None,
        )
        .await
        .unwrap();
    let expected_xpub_info = XPubInfo {
        paging: AddressInfoPaging {
            page: 1,
            total_pages: Some(1),
            items_on_page: 1000,
        },
        basic: XPubInfoBasic {
            address: xpub().to_string(),
            balance: Amount::from_sat(821),
            total_received: Amount::from_sat(821),
            total_sent: Amount::from_sat(0),
            unconfirmed_balance: Amount::from_sat(0),
            unconfirmed_txs: 0,
            txs: 1,
            secondary_value: None,
            used_tokens: 1,
            tokens: Some(vec![token()]),
        },
        txids: None,
        transactions: None,
    };
    assert_eq!(xpub_info, expected_xpub_info);
}

#[allow(clippy::too_many_lines)]
#[ignore]
#[tokio::test]
async fn test_xpub_info_entire_txs() {
    let xpub_info = blockbook()
        .xpub_info(xpub(), None, None, None, None, true, None, None)
        .await
        .unwrap();
    let expected_xpub_info = XPubInfo {
        paging: AddressInfoPaging {
            page: 1,
            total_pages: Some(1),
            items_on_page: 1000,
        },
        basic: XPubInfoBasic {
            address: xpub().to_string(),
            balance: Amount::from_sat(821),
            total_received: Amount::from_sat(821),
            total_sent: Amount::from_sat(0),
            unconfirmed_balance: Amount::from_sat(0),
            unconfirmed_txs: 0,
            txs: 1,
            secondary_value: None,
            used_tokens: 1,
            tokens: Some(vec![token()]),
        },
        txids: None,
        transactions: Some(vec![Transaction {
            txid: "d60691ecdf678eb3f88ebfaf315d3907d0be62ccd40fd1f027938249966f268d"
                .parse()
                .unwrap(),
            version: 2,
            lock_time: Some(Height::from_consensus(784_026).unwrap()),
            vin: vec![Vin {
                txid: "6c646dc364b600afcfc16ae44ee910cfee11a176a0d015cf423e453e3b3e9e16"
                    .parse()
                    .unwrap(),
                vout: Some(24),
                sequence: Some(Sequence(4_294_967_293)),
                n: 0,
                addresses: vec!["3AVvCje8AHCXrvgL8ZNsqUFdiu4LxmdNJk"
                    .parse::<Address<NetworkUnchecked>>()
                    .unwrap()
                    .assume_checked()],
                is_address: true,
                value: Amount::from_sat(137_000),
            }],
            vout: vec![
                Vout {
                    value: Amount::from_sat(135_146),
                    n: 0,
                    script: ScriptBuf::from_hex("a9143d46e616b172612000e823699338a5b0a9e6c6f387")
                        .unwrap(),
                    addresses: vec!["37H25Nx78ueHFhwr2KWGVcVu1155WLtb92"
                        .parse::<Address<NetworkUnchecked>>()
                        .unwrap()
                        .assume_checked()],
                    is_address: true,
                    spent: Some(true),
                },
                Vout {
                    value: Amount::from_sat(821),
                    n: 1,
                    script: ScriptBuf::from_hex("0014a778a9ec5d28761fcd88111b5b13d94714e2769a")
                        .unwrap(),
                    addresses: vec!["bc1q5au2nmza9pmplnvgzyd4ky7egu2wya56qa024u"
                        .parse::<Address<NetworkUnchecked>>()
                        .unwrap()
                        .assume_checked()],
                    is_address: true,
                    spent: None,
                },
            ],
            block_hash: Some("0000000000000000000036fe411dd9dd3ef7ce4531d65314e9ab73637dab3f68"
                .parse()
                .unwrap()),
            block_height: Some(Height::from_consensus(784_027).unwrap()),
            confirmations: xpub_info
                .transactions
                .as_ref()
                .unwrap()
                .get(0)
                .unwrap()
                .confirmations,
            block_time: "1680680375".parse().unwrap(),
            size: 375,
            vsize: 206,
            value: Amount::from_sat(135_967),
            value_in: Amount::from_sat(137_000),
            fees: Amount::from_sat(1_033),
            script:
                ScriptBuf::from_hex(
                    "02000000000101169e3e3b3e453e42cf15d0a076a111eecf10e94ee46ac1cfaf00b664c36d646c18000000232200206e0d706cbd560f4c9845604991ce15a47dea5e854f9b56ae11decf6d98a7a821fdffffff02ea0f02000000000017a9143d46e616b172612000e823699338a5b0a9e6c6f3873503000000000000160014a778a9ec5d28761fcd88111b5b13d94714e2769a03473044022018286becb369315af56ea7ab496588ace0e34e17b9c27b6ea06c65f794f07a4202202617528bfffcb75083700a86ee40551c25c8f62717ac962ce4733d033729abda014730440220276366c3cc6b1947d7be49ceb54c5ba6511ad74596119b7beeaf8b0bb21c267f02201a6e96d8735e08db6155b49e0ef4c9344c8f663a3dda3f795c718e43d7210294014e2102fab4f766e50937a07b96b045d12e210a28ca43570e9d46f8a6f6b3c385d5e7c7ad2102eef58d1fe659a43ef061b3646d51719f1005de7b74967786e75c5c6fb4e739a9ac73640380ca00b2689af60b00"
                    )
                    .unwrap(),
        }]),
    };
    assert_eq!(xpub_info, expected_xpub_info);
}

#[ignore]
#[tokio::test]
async fn test_xpub_info_basic_tokens() {
    let xpub_info = blockbook()
        .xpub_info_basic(xpub(), true, None, None)
        .await
        .unwrap();
    let expected_xpub_info = XPubInfoBasic {
        address: xpub().to_string(),
        balance: Amount::from_sat(821),
        total_received: Amount::from_sat(821),
        total_sent: Amount::from_sat(0),
        unconfirmed_balance: Amount::from_sat(0),
        unconfirmed_txs: 0,
        txs: 1,
        secondary_value: None,
        used_tokens: 1,
        tokens: Some(vec![token()]),
    };
    assert_eq!(xpub_info, expected_xpub_info);
}
