use blockbook::{
    websocket, Address, AddressFilter, Amount, Asset, Chain, Currency, Height, NetworkUnchecked,
    Sequence, Ticker, Time, Tx, TxDetail, Txid,
};
use std::str::FromStr;

async fn blockbook() -> blockbook::Client {
    blockbook::Client::new(
        format!("https://{}", std::env::var("BLOCKBOOK_SERVER").unwrap())
            .parse()
            .unwrap(),
    )
    .await
    .unwrap()
}

async fn blockbook_ws() -> websocket::Client {
    websocket::Client::new(
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

trait Redactor {
    fn redact_dynamic_address_info(self) -> Self;
    fn redact_confirmations(self) -> Self;
}

impl Redactor for insta::Settings {
    fn redact_dynamic_address_info(mut self) -> Self {
        self.add_redaction(".totalPages", "[REDACTED]");
        self.add_redaction(".balance", "[REDACTED]");
        self.add_redaction(".totalReceived", "[REDACTED]");
        self.add_redaction(".totalSent", "[REDACTED]");
        self.add_redaction(".unconfirmedBalance", "[REDACTED]");
        self.add_redaction(".unconfirmedTxs", "[REDACTED]");
        self.add_redaction(".txs", "[REDACTED]");
        self
    }

    fn redact_confirmations(mut self) -> Self {
        self.add_redaction(".confirmations", "[REDACTED]");
        self.add_redaction(".txs[].confirmations", "[REDACTED]");
        self.add_redaction(".transactions[].confirmations", "[REDACTED]");
        self
    }
}

#[ignore]
#[tokio::test]
async fn test_status() {
    let status = blockbook().await.status().await.unwrap();

    assert_eq!(status.blockbook.coin, Asset::Bitcoin);
    assert_eq!(status.blockbook.decimals, 8);
    assert_eq!(status.backend.chain, Chain::Main);
    assert_eq!(status.backend.protocol_version, "70016");
}

#[ignore]
#[tokio::test]
async fn test_block_hash() {
    let hash = blockbook()
        .await
        .block_hash(&Height::from_consensus(763_672).unwrap())
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

#[ignore]
#[tokio::test]
async fn test_tx() {
    let txid = "b0714235addd08daf83b979aa35cc9ed7558efb8327b86b4d3ccacd8b0482ae1"
        .parse()
        .unwrap();
    let tx = blockbook().await.transaction(&txid).await.unwrap();
    insta::Settings::new().redact_confirmations().bind(|| {
        insta::assert_json_snapshot!(tx);
    });
    let mut client = blockbook_ws().await;
    assert_eq!(client.transaction(txid).await.unwrap(), tx);
}

#[ignore]
#[tokio::test]
async fn test_lock_time() {
    let tx = blockbook()
        .await
        .transaction(
            &"bd99f123432e23aa8b88af0e9f701a4d6c8f0638dc133a14c7ccf57fb06596ac"
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
        .await
        .transaction(
            &"bd99f123432e23aa8b88af0e9f701a4d6c8f0638dc133a14c7ccf57fb06596ac"
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
        .await
        .transaction(
            &"c8d7b00135b9bd03055a8f47851eafae747b759b4608bd9f35e85b3285185679"
                .parse()
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tx.vin.get(0).unwrap().sequence, None);
}

#[ignore]
#[tokio::test]
async fn test_tx_specific() {
    let txid = "b0714235addd08daf83b979aa35cc9ed7558efb8327b86b4d3ccacd8b0482ae1"
        .parse()
        .unwrap();
    let tx = blockbook().await.transaction_specific(&txid).await.unwrap();
    insta::Settings::new().redact_confirmations().bind(|| {
        insta::assert_json_snapshot!(tx);
    });

    assert_eq!(
        blockbook_ws()
            .await
            .transaction_specific(txid)
            .await
            .unwrap(),
        tx
    );
}

#[ignore]
#[tokio::test]
async fn test_tx_specific_pre_segwit() {
    let tx = blockbook()
        .await
        .transaction_specific(
            &"0c5cb51f39ecb826cd477d94576abde1d2b6ef1b2e0ac7b9cea5d5ab28aba902"
                .parse()
                .unwrap(),
        )
        .await
        .unwrap();
    insta::Settings::new().redact_confirmations().bind(|| {
        insta::assert_json_snapshot!(tx);
    });
}

#[ignore]
#[tokio::test]
async fn test_block_by_hash() {
    let block = blockbook()
        .await
        .block_by_hash(
            &"000000000003ba27aa200b1cecaad478d2b00432346c3f1f3986da1afd33e506"
                .parse()
                .unwrap(),
        )
        .await
        .unwrap();
    insta::Settings::new().redact_confirmations().bind(|| {
        insta::assert_json_snapshot!(block);
    });
}

#[ignore]
#[tokio::test]
async fn test_block_by_height_with_opreturn_output() {
    let block = blockbook()
        .await
        .block_by_height(&Height::from_consensus(500_044).unwrap())
        .await
        .unwrap();
    insta::Settings::new().redact_confirmations().bind(|| {
        insta::assert_json_snapshot!(block);
    });
}

#[ignore]
#[tokio::test]
async fn test_tickers_list() {
    let tickers_list = blockbook()
        .await
        .tickers_list(&Time::from_consensus(1_674_821_349).unwrap())
        .await
        .unwrap();
    insta::assert_json_snapshot!(tickers_list);
}

#[ignore]
#[tokio::test]
async fn test_current_tickers_list() {
    let tickers_list = blockbook()
        .await
        .tickers_list(
            &Time::from_consensus(
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
    insta::assert_json_snapshot!(tickers_list, {".ts" => "[REDACTED]"});
}

#[ignore]
#[tokio::test]
async fn test_tickers() {
    // https://github.com/mitsuhiko/insta/blob/324cd7154a2f5e4b5198747049202f69305c2ac5/src/content/json.rs#L188
    #[derive(serde::Deserialize, serde::Serialize)]
    struct TestTicker {
        ts: Time,
        rates: std::collections::HashMap<String, f64>,
    }
    impl From<Ticker> for TestTicker {
        fn from(ticker: Ticker) -> Self {
            let rates = ticker
                .rates
                .into_iter()
                .map(|(currency, rate)| (serde_json::to_string(&currency).unwrap(), rate))
                .collect();
            Self {
                ts: ticker.timestamp,
                rates,
            }
        }
    }

    let tickers: TestTicker = blockbook()
        .await
        .tickers(Some(&Time::from_consensus(1_674_821_349).unwrap()))
        .await
        .unwrap()
        .into();
    insta::with_settings!({sort_maps => true}, {
        insta::assert_json_snapshot!(tickers);
    });
}

#[ignore]
#[tokio::test]
async fn test_current_tickers() {
    blockbook().await.tickers(None).await.unwrap();
}

#[ignore]
#[tokio::test]
async fn test_ticker() {
    let ticker = blockbook()
        .await
        .ticker(
            &Currency::Idr,
            Some(&Time::from_consensus(1_674_692_106).unwrap()),
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
    insta::assert_json_snapshot!(info, {
        ".bestHeight" => "[REDACTED]",
        ".bestHash" => "[REDACTED]",
    });
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
    insta::Settings::new().redact_confirmations().bind(|| {
        insta::assert_json_snapshot!(utxos.get(0).unwrap());
    });
}

#[ignore]
#[tokio::test]
async fn test_balance_history() {
    let client = blockbook().await;
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
                Some(&from),
                Some(&to),
                Some(&Currency::Chf),
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
                Some(&from),
                Some(&to),
                Some(&Currency::Chf),
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
    let address_info = blockbook().await.address_info(&addr_1()).await.unwrap();
    assert_eq!(&address_info.basic.address, &addr_1());
    assert_eq!(
        address_info.txids.unwrap().last().unwrap(),
        &Txid::from_str("98f08111f08baba3d33af28c74facc223a07d868c0568258980119761dea441d")
            .unwrap()
    );
}

#[ignore]
#[tokio::test]
async fn test_address_info_specific_no_args() {
    let address_info = blockbook()
        .await
        .address_info_specific(&addr_1(), None, None, None, None, None)
        .await
        .unwrap();
    assert_eq!(&address_info.basic.address, &addr_1());
    assert_eq!(
        address_info.txids.as_ref().unwrap().last().unwrap(),
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
        .await
        .address_info_specific_basic(&address, None)
        .await
        .unwrap()
        .txs;
    let address_info = blockbook()
        .await
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
        address_info.txids.as_ref().unwrap().get(0).unwrap(),
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
        .await
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
        .await
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
    insta::Settings::new()
        .redact_dynamic_address_info()
        .bind(|| {
            insta::assert_json_snapshot!(address_info);
        });
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

#[ignore]
#[tokio::test]
async fn test_address_info_specific_blocks_details() {
    let address_info = blockbook()
        .await
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
    insta::Settings::new()
        .redact_dynamic_address_info()
        .redact_confirmations()
        .bind(|| {
            insta::assert_json_snapshot!(address_info, {
                ".secondaryValue" => "[REDACTED]",
            });
        });
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
    assert_eq!(websocket_info, address_info);
}

#[ignore]
#[tokio::test]
async fn test_address_info_specific_blocks_details_light() {
    let address_info = blockbook()
        .await
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
    insta::Settings::new()
        .redact_dynamic_address_info()
        .redact_confirmations()
        .bind(|| {
            insta::assert_json_snapshot!(address_info);
        });
}

#[ignore]
#[tokio::test]
async fn test_address_info_correct_variant_full() {
    let address_info_full = blockbook()
        .await
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
        address_info_full.transactions.unwrap().get(0).unwrap(),
        Tx::Ordinary(..)
    ));
}

#[ignore]
#[tokio::test]
async fn test_address_info_correct_variant_light() {
    let address_info_light = blockbook()
        .await
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
        address_info_light.transactions.unwrap().get(0).unwrap(),
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
        .await
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
        .await
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
        .await
        .utxos_from_address(&counterparty_burner_addr(), false)
        .await
        .unwrap();
    insta::Settings::new().redact_confirmations().bind(|| {
        insta::assert_json_snapshot!(utxos.last().unwrap());
    });
}

fn xpub() -> &'static str {
    "zpub6qd36EtRVbyDyJToANn1vnuvhGenvepKnjeUzBXDk7JE4JYBxGUAPbjh22QqZ7JkRGuAtpgBfgKP1iT9GzgQxP1TgKPEBoN3e3vN3WtY2Su"
}

#[ignore]
#[tokio::test]
async fn test_utxos_from_xpub() {
    let utxos = blockbook()
        .await
        .utxos_from_xpub(xpub(), true)
        .await
        .unwrap();
    insta::Settings::new().redact_confirmations().bind(|| {
        insta::assert_json_snapshot!(utxos.get(0).unwrap());
    });
}

#[ignore]
#[tokio::test]
async fn test_xpub_info_basic_no_tokens() {
    let xpub_info = blockbook()
        .await
        .xpub_info_basic(xpub(), false, None, Some(&Currency::Btc))
        .await
        .unwrap();
    insta::assert_json_snapshot!(xpub_info);
}

#[ignore]
#[tokio::test]
async fn test_xpub_info_txids() {
    let xpub_info = blockbook()
        .await
        .xpub_info(xpub(), None, None, None, None, false, None, None)
        .await
        .unwrap();
    insta::assert_json_snapshot!(xpub_info);
}

#[ignore]
#[tokio::test]
async fn test_xpub_info_specific() {
    let xpub_info = blockbook()
        .await
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
    insta::assert_json_snapshot!(xpub_info);
}

#[ignore]
#[tokio::test]
async fn test_xpub_info_empty() {
    let xpub_info = blockbook()
        .await
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
    insta::assert_json_snapshot!(xpub_info);
}

#[ignore]
#[tokio::test]
async fn test_xpub_info_entire_txs() {
    let xpub_info = blockbook()
        .await
        .xpub_info(xpub(), None, None, None, None, true, None, None)
        .await
        .unwrap();
    insta::Settings::new().redact_confirmations().bind(|| {
        insta::assert_json_snapshot!(xpub_info);
    });
}

#[ignore]
#[tokio::test]
async fn test_xpub_info_basic_tokens() {
    let xpub_info = blockbook()
        .await
        .xpub_info_basic(xpub(), true, None, None)
        .await
        .unwrap();
    insta::assert_json_snapshot!(xpub_info);
}
