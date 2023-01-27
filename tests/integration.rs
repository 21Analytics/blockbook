use blockbook::{
    hashes::{self, hex::FromHex},
    Address, Amount, BlockHash, Height, PackedLockTime, ScriptPubKey, ScriptPubKeyType, ScriptSig,
    Sequence, Time, Transaction, TransactionSpecific, Vin, VinSpecific, Vout, VoutSpecific,
    Witness,
};
use std::str::FromStr;

static USED_BLOCKBOOK_COUNTER: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

static AVAILABLE_BLOCKBOOKS: once_cell::sync::Lazy<
    std::sync::Arc<std::sync::Mutex<Vec<blockbook::Blockbook>>>,
> = once_cell::sync::Lazy::new(|| {
    std::sync::Arc::new(std::sync::Mutex::new(blockbooks().collect()))
});

const TOTAL_BLOCKBOOKS: u8 = 4;

struct UsageCountingBlockbook {
    blockbook: blockbook::Blockbook,
}

impl std::ops::Deref for UsageCountingBlockbook {
    type Target = blockbook::Blockbook;

    fn deref(&self) -> &Self::Target {
        &self.blockbook
    }
}

impl Drop for UsageCountingBlockbook {
    fn drop(&mut self) {
        USED_BLOCKBOOK_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

fn blockbooks() -> impl Iterator<Item = blockbook::Blockbook> {
    (1..6).filter(|&i| i != 4).map(|i| {
        blockbook::Blockbook::new(url::Url::parse(&format!("https://btc{i}.trezor.io")).unwrap())
    })
}

async fn blockbook() -> UsageCountingBlockbook {
    loop {
        if let Some(blockbook) = AVAILABLE_BLOCKBOOKS.lock().unwrap().pop() {
            return UsageCountingBlockbook { blockbook };
        }
        if USED_BLOCKBOOK_COUNTER
            .compare_exchange(
                TOTAL_BLOCKBOOKS,
                0,
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
            )
            .is_ok()
        {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            let mut bb = AVAILABLE_BLOCKBOOKS.lock().unwrap();
            bb.extend(blockbooks());
            assert_eq!(bb.len(), TOTAL_BLOCKBOOKS as usize);
        }
        tokio::task::yield_now().await;
    }
}

#[tokio::test]
async fn test_block_hash() {
    let hash = blockbook().await.block_hash(763_672).await.unwrap();
    assert_eq!(
        hash.as_ref(),
        [
            0x4e, 0x67, 0xa2, 0x6f, 0x64, 0xcf, 0xbe, 0xef, 0x2c, 0x7f, 0x6a, 0x83, 0xd9, 0xa8,
            0x25, 0x51, 0x76, 0x25, 0x51, 0xcb, 0x1e, 0xbe, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ],
    );
}

#[tokio::test]
async fn test_btc_tx() {
    let txid = "b0714235addd08daf83b979aa35cc9ed7558efb8327b86b4d3ccacd8b0482ae1";
    let tx = blockbook().await.transaction(txid).await.unwrap();
    let expected_tx = Transaction{
        txid: txid.parse().unwrap(),
        version: 2,
        script: "0200000000010227fa15587fdefa0e4bddef8297e8e309478b6ecad3c79ccc4f11e0bf5a0ee4a60200000000ffffffff5a43b6f57bf76889924a806ae1cf2124de949c3f5acbeade82489f213837dde00000000000ffffffff034078f30100000000160014fff48015913acb35add9b3c74b5a6f76f2d145caf6c19c0000000000160014e1a8d8ab85131ec8a9521ad2e7daf16bcf3fe9e2187e36030000000016001431ac0b37f2dfa0a34c65e57c209361a916feb62d02483045022100e6bc8783be4d00222da4684b495c0e38578c72bc72d74321b776e777f430b5be02203dce27e811f0cb0b9ebad0d11d541a83344c985ec37a0273993caf582988c0a301210285b14271e50491ac26111dd42a6d9004f06a8e77355dac918c2fe7b1a7526ff002473044022036563b247efe66f50f453a6417d03bca152ad70913d7b69b29d7abcb602dd389022033f841a69c985ba457fb1c41a533fb0bce4b68a3bd42fdec60a89ab66623995901210285b14271e50491ac26111dd42a6d9004f06a8e77355dac918c2fe7b1a7526ff000000000".parse().unwrap(),
        block_hash: BlockHash::from_hash(hashes::sha256d::Hash::from_str(
                "00000000000000000006b7e2a7110c174f21633adbe955c8f86f36699bba6716"
        ).unwrap()),

    block_height: Height::from_consensus(765_165).unwrap(),
    confirmations: tx.confirmations,
    block_time: Time::from_consensus(1_669_723_092).unwrap(),
    value: Amount::from_sat(96_909_390),
    value_in: Amount::from_sat(96_935_907),
    fees: Amount::from_sat(26517),
    vin : vec![
        Vin {
            txid: "a6e40e5abfe0114fcc9cc7d3ca6e8b4709e3e89782efdd4b0efade7f5815fa27"
                .parse()
                .unwrap(),
            sequence: Sequence(4_294_967_295),
            n: 0,
            vout: Some(2),
            addresses: vec!["bc1qxxkqkdljm7s2xnr9u47zpymp4yt0ad3df4hudm"
                .parse::<Address>()
                .unwrap()],
            is_address: true,
            value: Amount::from_sat(59_231_084),
        },
        Vin {
            txid: "e0dd3738219f4882deeacb5a3f9c94de2421cfe16a804a928968f77bf5b6435a"
                .parse()
                .unwrap(),
            vout: None,
            sequence: Sequence(4_294_967_295),
            n: 1,
            addresses: vec!["bc1qxxkqkdljm7s2xnr9u47zpymp4yt0ad3df4hudm"
                .parse::<blockbook::Address>()
                .unwrap()],
            is_address: true,
            value: Amount::from_sat(37_704_823),
        },
    ],
    vout: vec![
        Vout {
            value: Amount::from_sat(32_733_248),
            n: 0,
            spent: Some(true),
            script: "0014fff48015913acb35add9b3c74b5a6f76f2d145ca".parse().unwrap(),
            addresses: vec!["bc1qll6gq9v38t9nttwek0r5kkn0wmedz3w2gshe0a"
                .parse::<Address>()
                .unwrap()],
            is_address: true,
        },
        Vout {
            value: Amount::from_sat(10_273_270),
            n: 1,
            spent: None,
            script: "0014e1a8d8ab85131ec8a9521ad2e7daf16bcf3fe9e2".parse().unwrap(),
            addresses: vec!["bc1qux5d32u9zv0v322jrtfw0kh3d08nl60z8q964g"
                .parse::<Address>()
                .unwrap()],
            is_address: true,
        },
        Vout {
            value: Amount::from_sat(53_902_872),
            n: 2,
            spent: Some(true),
            script: "001431ac0b37f2dfa0a34c65e57c209361a916feb62d".parse().unwrap(),
            addresses: vec!["bc1qxxkqkdljm7s2xnr9u47zpymp4yt0ad3df4hudm"
                .parse::<Address>()
                .unwrap()],
            is_address: true,
        },
    ],
    };
    assert_eq!(tx, expected_tx);
}

#[tokio::test]
async fn test_btc_tx_specific() {
    let txid = "b0714235addd08daf83b979aa35cc9ed7558efb8327b86b4d3ccacd8b0482ae1";
    let tx = blockbook()
        .await
        .transaction_btc_specific(txid)
        .await
        .unwrap();
    let expected_tx = TransactionSpecific{
        txid: txid.parse().unwrap(),
        version: 2,
        script: "0200000000010227fa15587fdefa0e4bddef8297e8e309478b6ecad3c79ccc4f11e0bf5a0ee4a60200000000ffffffff5a43b6f57bf76889924a806ae1cf2124de949c3f5acbeade82489f213837dde00000000000ffffffff034078f30100000000160014fff48015913acb35add9b3c74b5a6f76f2d145caf6c19c0000000000160014e1a8d8ab85131ec8a9521ad2e7daf16bcf3fe9e2187e36030000000016001431ac0b37f2dfa0a34c65e57c209361a916feb62d02483045022100e6bc8783be4d00222da4684b495c0e38578c72bc72d74321b776e777f430b5be02203dce27e811f0cb0b9ebad0d11d541a83344c985ec37a0273993caf582988c0a301210285b14271e50491ac26111dd42a6d9004f06a8e77355dac918c2fe7b1a7526ff002473044022036563b247efe66f50f453a6417d03bca152ad70913d7b69b29d7abcb602dd389022033f841a69c985ba457fb1c41a533fb0bce4b68a3bd42fdec60a89ab66623995901210285b14271e50491ac26111dd42a6d9004f06a8e77355dac918c2fe7b1a7526ff000000000".parse().unwrap(),
        wtxid: "5eb2c13ef559e9aee53e803e6a8abcbfc7bb60830e2bb7fec1437f98f463e889".parse().unwrap(),
        size: 402,
        time: Time::from_consensus(1_669_723_092).unwrap(),
        vsize: 240,
        weight: 957,
        blockhash: "00000000000000000006b7e2a7110c174f21633adbe955c8f86f36699bba6716".parse().unwrap(),
        blocktime: Time::from_consensus(1_669_723_092).unwrap(),
        confirmations: tx.confirmations,
        locktime: PackedLockTime::ZERO,
        vin : vec![
            VinSpecific {
                txid: "a6e40e5abfe0114fcc9cc7d3ca6e8b4709e3e89782efdd4b0efade7f5815fa27"
                    .parse()
                    .unwrap(),
                sequence: Sequence(4_294_967_295),
                vout: 2,
                tx_in_witness: Some(Witness::from_vec(vec![Vec::from_hex("3045022100e6bc8783be4d00222da4684b495c0e38578c72bc72d74321b776e777f430b5be02203dce27e811f0cb0b9ebad0d11d541a83344c985ec37a0273993caf582988c0a301").unwrap(), Vec::from_hex("0285b14271e50491ac26111dd42a6d9004f06a8e77355dac918c2fe7b1a7526ff0").unwrap()])),
                script_sig: Default::default(),
            },
            VinSpecific {
                txid: "e0dd3738219f4882deeacb5a3f9c94de2421cfe16a804a928968f77bf5b6435a"
                    .parse()
                    .unwrap(),
                sequence: Sequence(4_294_967_295),
                vout: 0,
                tx_in_witness: Some(Witness::from_vec(vec![Vec::from_hex("3044022036563b247efe66f50f453a6417d03bca152ad70913d7b69b29d7abcb602dd389022033f841a69c985ba457fb1c41a533fb0bce4b68a3bd42fdec60a89ab66623995901").unwrap(), Vec::from_hex("0285b14271e50491ac26111dd42a6d9004f06a8e77355dac918c2fe7b1a7526ff0").unwrap()])),
                script_sig: Default::default(),
            },
        ],
        vout: vec![
            VoutSpecific {
                value: Amount::from_sat(32_733_248),
                n: 0,
                script_pub_key: ScriptPubKey {
                    address: "bc1qll6gq9v38t9nttwek0r5kkn0wmedz3w2gshe0a"
                        .parse::<blockbook::Address>()
                        .unwrap(),
                    asm: "0 fff48015913acb35add9b3c74b5a6f76f2d145ca".into(),
                    desc: Some("addr(bc1qll6gq9v38t9nttwek0r5kkn0wmedz3w2gshe0a)#k54tyxd5".into()),
                    script: "0014fff48015913acb35add9b3c74b5a6f76f2d145ca".parse().unwrap(),
                    r#type: ScriptPubKeyType::WitnessV0PubKeyHash,
                },
            },
            VoutSpecific {
                value: Amount::from_sat(10_273_270),
                n: 1,
                script_pub_key: ScriptPubKey {
                    address: "bc1qux5d32u9zv0v322jrtfw0kh3d08nl60z8q964g"
                        .parse::<blockbook::Address>()
                        .unwrap(),
                    asm: "0 e1a8d8ab85131ec8a9521ad2e7daf16bcf3fe9e2".into(),
                    desc: Some("addr(bc1qux5d32u9zv0v322jrtfw0kh3d08nl60z8q964g)#f20g3jce".into()),
                    script: "0014e1a8d8ab85131ec8a9521ad2e7daf16bcf3fe9e2".parse().unwrap(),
                    r#type: ScriptPubKeyType::WitnessV0PubKeyHash,
                },
            },
            VoutSpecific {
                value: Amount::from_sat(53_902_872),
                n: 2,
                script_pub_key: ScriptPubKey {
                    address: "bc1qxxkqkdljm7s2xnr9u47zpymp4yt0ad3df4hudm"
                        .parse::<blockbook::Address>()
                        .unwrap(),
                    asm: "0 31ac0b37f2dfa0a34c65e57c209361a916feb62d".into(),
                    desc: Some("addr(bc1qxxkqkdljm7s2xnr9u47zpymp4yt0ad3df4hudm)#mxf8kxkq".into()),
                    script: "001431ac0b37f2dfa0a34c65e57c209361a916feb62d".parse().unwrap(),
                    r#type: ScriptPubKeyType::WitnessV0PubKeyHash,
                },
            },
        ],
    };
    assert_eq!(tx, expected_tx);
}

#[tokio::test]
async fn test_btc_tx_specific_pre_segwit() {
    let txid = "0c5cb51f39ecb826cd477d94576abde1d2b6ef1b2e0ac7b9cea5d5ab28aba902";
    let tx = blockbook()
        .await
        .transaction_btc_specific(txid)
        .await
        .unwrap();
    let expected_tx = TransactionSpecific{
        txid: txid.parse().unwrap(),
        version: 1,
        script: "01000000013cdefb50d22666b59b24f047b019e09a2c077ad0fb8febda33a5e0bad45990e2000000006a47304402202015dfc5b5d9030f9538c1f6e0b99fe8dbf46260044e45ad2f883744292af09b0220066353c0d19f9734278ba7072fa8b3ba1c1c30bdd583721439b8ee375a098ad8012103de2010f23c4eda698d373cfc8f7ecd576fbb4e40f67a8634ac007bb4b80a4fd4ffffffff01e62e1900000000001976a914029f45cefe259733c9d860b70f7a8385596607bf88ac00000000".parse().unwrap(),
        wtxid: txid.parse().unwrap(),
        size: 191,
        time: Time::from_consensus(1_513_622_125).unwrap(),
        vsize: 191,
        weight: 764,
        blockhash: "00000000000000000024fb37364cbf81fd49cc2d51c09c75c35433c3a1945d04".parse().unwrap(),
        blocktime: Time::from_consensus(1_513_622_125).unwrap(),
        confirmations: tx.confirmations,
        locktime: PackedLockTime::ZERO,
        vin : vec![
            VinSpecific {
                txid: "e29059d4bae0a533daeb8ffbd07a072c9ae019b047f0249bb56626d250fbde3c"
                    .parse()
                    .unwrap(),
                sequence: Sequence(4_294_967_295),
                vout: 0,
                tx_in_witness: None,
                script_sig: ScriptSig {
                    asm: "304402202015dfc5b5d9030f9538c1f6e0b99fe8dbf46260044e45ad2f883744292af09b0220066353c0d19f9734278ba7072fa8b3ba1c1c30bdd583721439b8ee375a098ad8[ALL] 03de2010f23c4eda698d373cfc8f7ecd576fbb4e40f67a8634ac007bb4b80a4fd4".into(),
                    script: "47304402202015dfc5b5d9030f9538c1f6e0b99fe8dbf46260044e45ad2f883744292af09b0220066353c0d19f9734278ba7072fa8b3ba1c1c30bdd583721439b8ee375a098ad8012103de2010f23c4eda698d373cfc8f7ecd576fbb4e40f67a8634ac007bb4b80a4fd4".parse().unwrap(),
                },
            },
        ],
        vout: vec![
            VoutSpecific {
                value: Amount::from_sat(1_650_406),
                n: 0,
                script_pub_key: ScriptPubKey {
                    address: "1Es9qximvz5W9TqtZfxx9cV7thmvDutWf"
                        .parse::<blockbook::Address>()
                        .unwrap(),
                    asm: "OP_DUP OP_HASH160 029f45cefe259733c9d860b70f7a8385596607bf OP_EQUALVERIFY OP_CHECKSIG".into(),
                    desc: Some("addr(1Es9qximvz5W9TqtZfxx9cV7thmvDutWf)#4h8xrdj5".into()),
                    script: "76a914029f45cefe259733c9d860b70f7a8385596607bf88ac".parse().unwrap(),
                    r#type: ScriptPubKeyType::PubKeyHash,
                },
            },
        ],
    };
    assert_eq!(tx, expected_tx);
}
