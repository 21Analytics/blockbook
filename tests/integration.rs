use blockbook::{
    hashes, Address, Amount, BlockHash, Height, Sequence, Time, Transaction, Vin, Vout,
};
use std::str::FromStr;

fn blockbook(i: u8) -> blockbook::Blockbook {
    blockbook::Blockbook::new(url::Url::parse(&format!("https://btc{i}.trezor.io")).unwrap())
}

#[tokio::test]
async fn test_block_hash() {
    let hash = blockbook(1).block_hash(763_672).await.unwrap();
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
    let tx = blockbook(2).transaction(txid).await.unwrap();
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
