use bdk::bitcoin::consensus::Decodable;
use bdk::bitcoin::hashes::Hash as LegacyHash;
use bdk::chain::{
    bitcoin::{OutPoint, ScriptBuf, Txid},
    local_chain::CheckPoint,
    BlockId, ConfirmationTimeAnchor, TxGraph,
};
use bitcoin::consensus::Encodable;
use bitcoin::hashes::Hash;
use futures::{stream::FuturesUnordered, StreamExt, TryStreamExt};
use std::collections::BTreeMap;

fn tx_to_legacy_tx(tx: &bitcoin::Transaction) -> bdk::bitcoin::Transaction {
    let buf = &mut Vec::with_capacity(tx.total_size());
    tx.consensus_encode(buf).expect("consensus encoding failed");
    bdk::bitcoin::Transaction::consensus_decode(&mut buf.as_slice())
        .expect("consensus decoding failed")
}

impl crate::Client {
    /// Prepare a [`LocalChain`] update with blocks fetched from Blockbook.
    ///
    /// * `local_tip` is the previous tip of [`LocalChain::tip`].
    /// * `heights` is the block heights that we are interested in fetching from Blockbook.
    ///
    /// The result of this method can be applied to [`LocalChain::apply_update`].
    ///
    /// [`LocalChain`]: bdk::chain::local_chain::LocalChain
    /// [`LocalChain::tip`]: bdk::chain::local_chain::LocalChain::tip
    /// [`LocalChain::apply_update`]: bdk::chain::local_chain::LocalChain::apply_update
    #[allow(clippy::too_many_lines)]
    pub async fn update_local_chain(
        &self,
        local_tip: Option<CheckPoint>,
        heights: impl IntoIterator<IntoIter = impl Iterator<Item = u32> + Send> + Send,
    ) -> Result<bdk::chain::local_chain::Update, crate::Error> {
        let new_tip_height = self
            .status()
            .await?
            .blockbook
            .best_height
            .to_consensus_u32();

        let heights_to_fetch: std::collections::HashSet<_> = heights
            .into_iter()
            .map(|h| {
                if h <= new_tip_height {
                    Ok(h)
                } else {
                    Err(crate::Error::BdkError(format!(
                        "requested height {h} exceeds remote tip at {new_tip_height}"
                    )))
                }
            })
            .collect::<Result<Vec<u32>, _>>()?
            .into_iter()
            // ensure that the returned `local_chain::Update` contains a view of
            // the chain tip that is reasonably reorg-resistant
            .chain(new_tip_height - 10..=new_tip_height)
            .collect();

        let mut fetched_blocks = heights_to_fetch
            .into_iter()
            .map(|height| async move {
                self.block_hash(&crate::Height::from_consensus(height).unwrap())
                    .await
                    .map(|block_hash| {
                        (
                            height,
                            bdk::bitcoin::BlockHash::from_byte_array(block_hash.to_byte_array()),
                        )
                    })
            })
            .collect::<FuturesUnordered<_>>()
            .try_collect::<BTreeMap<u32, bdk::bitcoin::BlockHash>>()
            .await?;

        // find the earliest point of agreement between local chain and fetched chain
        let earliest_agreement_cp = 'agreement: {
            let Some(local_tip) = local_tip else {
                break 'agreement None;
            };

            let local_tip_height = local_tip.height();
            for local_cp in local_tip.iter() {
                let local_block = local_cp.block_id();

                // the updated hash (block hash at this height after the update), can either be:
                // 1. a block that already existed in `fetched_blocks`
                // 2. a block that exists locally and at least has a depth of assume_final_depth
                // 3. otherwise we can freshly fetch the block from remote, which is safe as it
                //    is guaranteed that this would be at or below assume_final_depth from the
                //    remote tip
                let assume_final_depth: u32 = 15;
                let updated_hash = match fetched_blocks.entry(local_block.height) {
                    std::collections::btree_map::Entry::Occupied(entry) => *entry.get(),
                    std::collections::btree_map::Entry::Vacant(entry) => *entry.insert(
                        if local_tip_height - local_block.height >= assume_final_depth {
                            local_block.hash
                        } else {
                            let block_hash = self
                                .block_hash(
                                    &crate::Height::from_consensus(local_block.height).unwrap(),
                                )
                                .await?;
                            bdk::bitcoin::BlockHash::from_byte_array(block_hash.to_byte_array())
                        },
                    ),
                };

                // since we may introduce blocks below the point of agreement, we cannot break
                // here unconditionally - we only break if we guarantee there are no new heights
                // below our current local checkpoint
                if local_block.hash == updated_hash {
                    let first_new_height = *fetched_blocks
                        .keys()
                        .next()
                        .expect("must have at least one new block");
                    if first_new_height >= local_block.height {
                        break 'agreement Some(local_cp);
                    }
                }
            }
            None
        };

        // first checkpoint to use for the update chain
        let first_cp = if let Some(cp) = earliest_agreement_cp {
            cp
        } else {
            let (&height, &hash) = fetched_blocks
                .iter()
                .next()
                .expect("must have at least one new block");
            CheckPoint::new(BlockId {
                height,
                hash: bdk::bitcoin::BlockHash::from_byte_array(hash.to_byte_array()),
            })
        };
        // we exclude anything at or below the first cp of the update chain otherwise
        // building the chain will fail
        let cutoff = first_cp.height() + 1;
        // transform fetched chain into the update chain
        let tip = first_cp
            .extend(
                fetched_blocks
                    .split_off(&cutoff)
                    .into_iter()
                    .map(|(height, hash)| BlockId {
                        height,
                        hash: bdk::bitcoin::BlockHash::from_byte_array(hash.to_byte_array()),
                    }),
            )
            .expect("must extend checkpoint");

        Ok(bdk::chain::local_chain::Update {
            tip,
            introduce_older_blocks: true,
        })
    }

    /// Scan Blockbook for the data specified and return a [`TxGraph`] and a map of last active
    /// indices.
    ///
    /// * `keychain_script_pubkeys`: keychains that we want to scan transactions for
    /// * `txids`: transactions for which we want updated [`ConfirmationTimeAnchor`]s
    /// * `outpoints`: transactions associated with these outpoints (residing, spending) that we
    ///     want to include in the update
    ///
    /// The scan for each keychain stops after a gap of `stop_gap` script pubkeys with no associated
    /// transactions. `parallel_requests` specifies the max number of HTTP requests to make in
    /// parallel.
    pub async fn scan_txs_with_keychains<K: Ord + Clone + Send>(
        &self,
        keychain_script_pubkeys: BTreeMap<
            K,
            impl IntoIterator<IntoIter = impl Iterator<Item = (u32, ScriptBuf)> + Send> + Send,
        >,
        network: &bdk::bitcoin::Network,
        additional_txids: impl IntoIterator<IntoIter = impl Iterator<Item = Txid> + Send> + Send,
        additional_outpoints: impl IntoIterator<IntoIter = impl Iterator<Item = OutPoint> + Send> + Send,
        stop_gap: u32,
        parallel_requests: usize,
    ) -> Result<(TxGraph<ConfirmationTimeAnchor>, BTreeMap<K, u32>), crate::Error> {
        let mut graph = TxGraph::<ConfirmationTimeAnchor>::default();

        let last_active_indexes = self
            .process_keychain_script_pubkeys(
                &mut graph,
                keychain_script_pubkeys,
                network,
                stop_gap,
                parallel_requests,
            )
            .await?;

        self.process_txids(&mut graph, additional_txids, parallel_requests)
            .await?;

        self.process_outpoints(&mut graph, additional_outpoints, parallel_requests)
            .await?;

        Ok((graph, last_active_indexes))
    }

    /// Convenience method to call `update_tx_graph` without requiring keychains.
    pub async fn scan_txs(
        &self,
        misc_script_pubkeys: impl IntoIterator<IntoIter = impl Iterator<Item = ScriptBuf> + Send> + Send,
        network: &bdk::bitcoin::Network,
        additional_txids: impl IntoIterator<IntoIter = impl Iterator<Item = Txid> + Send> + Send,
        additional_outpoints: impl IntoIterator<IntoIter = impl Iterator<Item = OutPoint> + Send> + Send,
        parallel_requests: usize,
    ) -> Result<TxGraph<ConfirmationTimeAnchor>, crate::Error> {
        self.scan_txs_with_keychains(
            [(
                (),
                misc_script_pubkeys
                    .into_iter()
                    .enumerate()
                    // the number of script pubkeys will never reach `u32::MAX`
                    .map(|(i, spk)| (u32::try_from(i).unwrap(), spk)),
            )]
            .into(),
            network,
            additional_txids,
            additional_outpoints,
            u32::MAX,
            parallel_requests,
        )
        .await
        .map(|(g, _)| g)
    }

    async fn process_keychain_script_pubkeys<K: Ord + Clone + Send>(
        &self,
        graph: &mut TxGraph<ConfirmationTimeAnchor>,
        keychain_script_pubkeys: BTreeMap<
            K,
            impl IntoIterator<IntoIter = impl Iterator<Item = (u32, ScriptBuf)> + Send> + Send,
        >,
        network: &bdk::bitcoin::Network,
        stop_gap: u32,
        parallel_requests: usize,
    ) -> Result<BTreeMap<K, u32>, crate::Error> {
        let parallel_requests = Ord::max(parallel_requests, 1);
        let mut last_active_indexes = BTreeMap::<K, u32>::new();

        for (keychain, script_pubkeys) in keychain_script_pubkeys {
            let mut script_pubkeys =
                futures::stream::iter(script_pubkeys.into_iter()).chunks(parallel_requests);
            let mut last_index = Option::<u32>::None;
            let mut last_active_index = Option::<u32>::None;

            let mut txids = Vec::new();
            while let Some(spks) = script_pubkeys.next().await {
                let mut handles = spks
                    .into_iter()
                    .map(|(spk_index, spk)| async move {
                        let address = bitcoin::Address::new(
                            match network {
                                bdk::bitcoin::Network::Bitcoin => bitcoin::Network::Bitcoin,
                                bdk::bitcoin::Network::Testnet => bitcoin::Network::Testnet,
                                bdk::bitcoin::Network::Signet => bitcoin::Network::Signet,
                                bdk::bitcoin::Network::Regtest => bitcoin::Network::Regtest,
                                _=> panic!("unsupported network")
                            },
                            bitcoin::address::Payload::from_script(bitcoin::Script::from_bytes(spk.as_bytes())).map_err(|e| crate::Error::BdkError(format!(
                                "error constructing bitcoin address payload from script pubkey {spk}: {e:?}"
                            )))?,
                        );
                        self.address_info(&address)
                            .await
                            .map(|info| (spk_index, info.txids))
                    })
                    .collect::<futures::stream::FuturesOrdered<_>>();

                while let Some((index, fresh_txids)) = handles.try_next().await? {
                    last_index = Some(index);
                    let Some(ids) = fresh_txids else {
                        continue;
                    };
                    if !ids.is_empty() {
                        last_active_index = Some(index);
                    }
                    txids.extend(ids);
                }

                if last_index > last_active_index.map(|i| i.saturating_add(stop_gap)) {
                    break;
                }
            }

            if let Some(last_active_index) = last_active_index {
                last_active_indexes.insert(keychain, last_active_index);
            }

            let mut handles = txids
                .iter()
                .map(|txid| async {
                    let tx = self.transaction_specific(txid).await?;
                    let tx_anchor_info = if tx.blockhash.is_some() {
                        extract_anchor_info(&self.transaction(txid).await?)
                    } else {
                        None
                    };
                    Ok::<_, crate::Error>((tx, tx_anchor_info))
                })
                .collect::<FuturesUnordered<_>>()
                .map(|fetched| {
                    let (tx, tx_anchor_info) = fetched?;
                    if let Some(anchor) = tx_anchor_info {
                        let _ = graph.insert_anchor(
                            bdk::bitcoin::Txid::from_byte_array(tx.txid.to_byte_array()),
                            anchor,
                        );
                    }
                    let _ = graph.insert_tx(tx_to_legacy_tx(&tx.into()));
                    Ok::<_, crate::Error>(())
                })
                .chunks(parallel_requests);
            while let Some(handle) = handles.next().await {
                handle.into_iter().collect::<Result<_, _>>()?;
            }
        }
        Ok(last_active_indexes)
    }

    async fn process_txids(
        &self,
        graph: &mut TxGraph<ConfirmationTimeAnchor>,
        txids: impl IntoIterator<IntoIter = impl Iterator<Item = Txid> + Send> + Send,
        parallel_requests: usize,
    ) -> Result<(), crate::Error> {
        let mut handles = txids
            .into_iter()
            .filter(|txid| graph.get_tx(*txid).is_none())
            .map(|txid| async move {
                self.transaction(&bitcoin::Txid::from_byte_array(txid.to_byte_array()))
                    .await
            })
            .collect::<FuturesUnordered<_>>()
            .map(|fetched| {
                let tx = fetched?;
                if let Some(anchor) = extract_anchor_info(&tx) {
                    let _ = graph.insert_anchor(
                        bdk::bitcoin::Txid::from_byte_array(tx.txid.to_byte_array()),
                        anchor,
                    );
                }
                Ok::<_, crate::Error>(())
            })
            .chunks(parallel_requests);
        while let Some(handle) = handles.next().await {
            handle.into_iter().collect::<Result<_, _>>()?;
        }
        Ok(())
    }

    async fn add_tx_anchor(
        &self,
        graph: &mut TxGraph<ConfirmationTimeAnchor>,
        txid: &bdk::bitcoin::Txid,
    ) -> Result<(), crate::Error> {
        let tx = self
            .transaction_specific(&bitcoin::Txid::from_byte_array(txid.to_byte_array()))
            .await?;
        // skip mempool transactions
        if tx.blockhash.is_some() {
            if let Some(anchor) = extract_anchor_info(&self.transaction(&tx.txid).await?) {
                let _ = graph.insert_anchor(
                    bdk::bitcoin::Txid::from_byte_array(tx.txid.to_byte_array()),
                    anchor,
                );
            }
        }
        let _ = graph.insert_tx(tx_to_legacy_tx(&tx.into()));
        Ok(())
    }

    async fn process_outpoints(
        &self,
        graph: &mut TxGraph<ConfirmationTimeAnchor>,
        outpoints: impl IntoIterator<IntoIter = impl Iterator<Item = OutPoint> + Send> + Send,
        parallel_requests: usize,
    ) -> Result<(), crate::Error> {
        let mut outpoints = futures::stream::iter(outpoints.into_iter()).chunks(parallel_requests);
        while let Some(outpoints) = outpoints.next().await {
            let mut handles = outpoints
                .into_iter()
                .map(|op| async move {
                    self.transaction(&bitcoin::Txid::from_byte_array(op.txid.to_byte_array()))
                        .await
                        .map(|tx| (op, tx))
                })
                .collect::<FuturesUnordered<_>>();

            while let Some((op, tx)) = handles.try_next().await? {
                if graph.get_tx(op.txid).is_none() {
                    self.add_tx_anchor(graph, &op.txid).await?;
                }

                if let Some(id) = tx
                    .vout
                    .get(usize::try_from(op.vout).unwrap())
                    .unwrap()
                    .spent_tx_id
                {
                    if graph
                        .get_tx(bdk::bitcoin::Txid::from_byte_array(id.to_byte_array()))
                        .is_none()
                    {
                        self.add_tx_anchor(
                            graph,
                            &bdk::bitcoin::Txid::from_byte_array(id.to_byte_array()),
                        )
                        .await?;
                    }
                }
            }
        }
        Ok(())
    }
}

fn extract_anchor_info(tx: &crate::Transaction) -> Option<ConfirmationTimeAnchor> {
    let height = tx.block_height?.to_consensus_u32();
    Some(ConfirmationTimeAnchor {
        anchor_block: BlockId {
            height,
            hash: bdk::bitcoin::BlockHash::from_byte_array(tx.block_hash?.to_byte_array()),
        },
        confirmation_height: height,
        confirmation_time: tx.block_time.to_consensus_u32().into(),
    })
}

#[cfg(test)]
mod tests {
    #[ignore]
    #[tokio::test]
    async fn test_xpub_update() {
        let descriptor = "wpkh(xpub6BxWUuYbCEtGGi5ZVfCmWcivMLMu3QqKxWc3RPjSz6YTx6ujSx939URQycVfZHzubzfZPsV4kMcHF9E1qbrPMueFwdzP1yj56bo5GKm5RPm/0/*)";
        let mut wallet =
            bdk::Wallet::new_no_persist(descriptor, None, bdk::bitcoin::Network::Bitcoin).unwrap();
        let client = crate::Client::new(
            format!("https://{}", std::env::var("BLOCKBOOK_SERVER").unwrap())
                .parse()
                .unwrap(),
        )
        .await
        .unwrap();
        let (update_graph, last_active_indices) = client
            .scan_txs_with_keychains(
                wallet.spks_of_all_keychains(),
                &bdk::bitcoin::Network::Bitcoin,
                None,
                None,
                1,
                10,
            )
            .await
            .unwrap();
        let txid = "d60691ecdf678eb3f88ebfaf315d3907d0be62ccd40fd1f027938249966f268d"
            .parse()
            .unwrap();
        let anchors: Vec<_> = update_graph.all_anchors().iter().collect();
        assert_eq!(anchors.len(), 1);
        assert_eq!(
            anchors.get(0).unwrap(),
            &&(
                bdk::chain::ConfirmationTimeAnchor {
                    anchor_block: bdk::chain::BlockId {
                        height: 784_027,
                        hash: "0000000000000000000036fe411dd9dd3ef7ce4531d65314e9ab73637dab3f68"
                            .parse()
                            .unwrap(),
                    },
                    confirmation_height: 784_027,
                    confirmation_time: 1_680_680_375,
                },
                txid
            )
        );

        let tx_outs = update_graph.all_txouts().collect::<Vec<_>>();
        let (outpoint, txout) = tx_outs.get(1).unwrap();
        assert_eq!(outpoint.txid, txid);
        let balance = 821;
        assert_eq!(txout.value, balance);
        assert_eq!(
            *last_active_indices
                .get(&bdk::KeychainKind::External)
                .unwrap(),
            0
        );
        assert!(last_active_indices
            .get(&bdk::KeychainKind::Internal)
            .is_none());

        let update = bdk::wallet::Update {
            last_active_indices,
            graph: update_graph,
            chain: None,
        };
        wallet.apply_update(update).unwrap();
        wallet.commit().unwrap();
        assert_eq!(wallet.get_balance().total(), balance);
    }
}
