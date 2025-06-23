use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use clarity::consts::STACKS_EPOCH_MAX;
use clarity::vm::costs::ExecutionCost;
use stacks_common::types::chainstate::{BurnchainHeaderHash, TrieHash};

use crate::burnchains::bitcoin::Error as bitcoin_error;
use crate::burnchains::tests::test_doubles::{
    BurnchainIndexerTestDouble, MockBlockParser, MockDownloader, MockIndexer, StubBlock,
    TestBlockIPC, TestHeaderIPC,
};
use crate::burnchains::{Burnchain, BurnchainBlock, Error as burnchain_error};
use crate::chainstate::coordinator::comm::CoordinatorCommunication;
use crate::core::{EpochList, StacksEpoch, StacksEpochId};

#[derive(Clone)]
struct TestErrorConfig {
    download_error: bool,
    parse_error: bool,
    db_error: bool,
}

fn default_execution_cost() -> ExecutionCost {
    ExecutionCost {
        write_length: 0,
        write_count: 0,
        read_length: 0,
        read_count: 0,
        runtime: 0,
    }
}

fn build_epoch_list() -> EpochList {
    let epoch = StacksEpoch {
        epoch_id: StacksEpochId::Epoch20,
        start_height: 0,
        end_height: STACKS_EPOCH_MAX,
        block_limit: default_execution_cost(),
        network_epoch: 2,
    };
    EpochList::new(&[epoch])
}

fn build_blocks() -> Vec<StubBlock> {
    vec![
        StubBlock::new(0, BurnchainHeaderHash::zero()),
        StubBlock::new(
            1,
            BurnchainHeaderHash::from_test_data(1, &TrieHash::from_empty_data(), 0),
        ),
        StubBlock::new(
            2,
            BurnchainHeaderHash::from_test_data(2, &TrieHash::from_empty_data(), 0),
        ),
    ]
}

fn build_headers(blocks: &[StubBlock]) -> Vec<TestHeaderIPC> {
    blocks
        .iter()
        .map(|b| TestHeaderIPC {
            height: b.height,
            hash: b.hash.as_ref().try_into().unwrap(),
        })
        .collect()
}

fn setup_indexer(
    blocks: &[StubBlock],
    headers: &[TestHeaderIPC],
    config: Option<TestErrorConfig>,
) -> BurnchainIndexerTestDouble {
    let mut td = BurnchainIndexerTestDouble::with_components(MockIndexer::new());

    let headers = headers.to_vec();
    td.indexer
        .expect_get_headers_path()
        .return_const("/tmp/stacks-test".to_string());
    td.indexer
        .expect_sync_headers()
        .returning(|_, end| Ok(end.unwrap_or(3)));
    td.indexer
        .expect_get_first_block_header_hash()
        .returning(|| Ok(BurnchainHeaderHash::zero()));
    td.indexer
        .expect_get_first_block_header_timestamp()
        .returning(|| Ok(0));
    td.indexer
        .expect_get_stacks_epochs()
        .returning(build_epoch_list);
    td.indexer
        .expect_get_highest_header_height()
        .returning(|| Ok(100));
    td.indexer.expect_find_chain_reorg().returning(|| Ok(0));
    td.indexer
        .expect_read_headers()
        .returning(move |_, _| Ok(headers.clone()));

    let cfg = config.clone();
    td.indexer.expect_downloader().returning(move || {
        let mut mock = MockDownloader::new();
        match cfg.clone() {
            Some(TestErrorConfig {
                download_error: true,
                ..
            }) => {
                mock.expect_download().returning(|_: &TestHeaderIPC| {
                    Err(burnchain_error::DownloadError(
                        bitcoin_error::ConnectionError,
                    ))
                });
            }
            _ => {
                mock.expect_download().returning(|h: &TestHeaderIPC| {
                    Ok(TestBlockIPC {
                        header: TestHeaderIPC {
                            height: h.height,
                            hash: h.hash,
                        },
                        data: vec![],
                    })
                });
            }
        }
        mock
    });

    let cfg2 = config.clone();
    td.indexer.expect_parser().returning(move || {
        let mut mock = MockBlockParser::new();
        match cfg2.clone() {
            Some(TestErrorConfig {
                parse_error: true, ..
            }) => {
                mock.expect_parse()
                    .returning(|_, _| Err(burnchain_error::ParseError));
            }
            _ => {
                mock.expect_parse().returning(|b: &TestBlockIPC, _| {
                    let h = BurnchainHeaderHash::from(b.header.hash);
                    let s = StubBlock::new(b.header.height, h);
                    Ok(BurnchainBlock::Bitcoin(s.to_block()))
                });
            }
        }
        mock
    });

    let clone = td.clone();
    td.indexer
        .expect_reader()
        .returning(move || clone.indexer.clone());

    td
}

#[test]
fn test_sync_with_indexer_happy_path() {
    let hash = BurnchainHeaderHash::zero();
    let mut bc = Burnchain::default_unittest(0, &hash);
    let blocks = build_blocks();
    let headers = build_headers(&blocks);
    let mut td = setup_indexer(&blocks, &headers, None);

    let (_, ch) = CoordinatorCommunication::instantiate();
    let keep_running = Some(Arc::new(AtomicBool::new(true)));
    let res = bc.sync_with_indexer(&mut td, ch, Some(2), None, keep_running);

    assert!(res.is_ok(), "Expected ok, got: {:?}", res.err());
    assert_eq!(res.unwrap().block_height, 2);
}

#[test]
fn test_sync_with_indexer_download_failure() {
    let hash = BurnchainHeaderHash::zero();
    let mut bc = Burnchain::default_unittest(0, &hash);
    let blocks = build_blocks();
    let headers = build_headers(&blocks);
    let config = TestErrorConfig {
        download_error: true,
        parse_error: false,
        db_error: false,
    };
    let mut td = setup_indexer(&blocks, &headers, Some(config));

    let (_, ch) = CoordinatorCommunication::instantiate();
    let keep_running = Some(Arc::new(AtomicBool::new(true)));
    let res = bc.sync_with_indexer(&mut td, ch, Some(2), None, keep_running);

    assert!(res.is_err(), "Expected err, got ok");
    assert!(matches!(res, Err(burnchain_error::DownloadError(_))));
}

#[test]
fn test_sync_with_indexer_error_precedence() {
    let tests = [
        TestErrorConfig {
            download_error: true,
            parse_error: false,
            db_error: false,
        },
        TestErrorConfig {
            download_error: false,
            parse_error: true,
            db_error: false,
        },
        TestErrorConfig {
            download_error: false,
            parse_error: false,
            db_error: true,
        },
        TestErrorConfig {
            download_error: true,
            parse_error: true,
            db_error: false,
        },
        TestErrorConfig {
            download_error: true,
            parse_error: true,
            db_error: true,
        },
    ];

    for cfg in tests.iter().cloned() {
        let hash = BurnchainHeaderHash::zero();
        let mut bc = Burnchain::default_unittest(0, &hash);
        let blocks = build_blocks();
        let headers = build_headers(&blocks);
        let mut td = setup_indexer(&blocks, &headers, Some(cfg.clone()));

        let (_, ch) = CoordinatorCommunication::instantiate();
        let keep_running = Some(Arc::new(AtomicBool::new(true)));
        let res = bc.sync_with_indexer(&mut td, ch, Some(2), None, keep_running);

        if cfg.db_error && !cfg.download_error && !cfg.parse_error {
            assert!(res.is_ok(), "Expected ok, got err");
        } else {
            assert!(res.is_err(), "Expected err, got ok");
            match res {
                Err(burnchain_error::DownloadError(_)) => {
                    assert!(cfg.download_error);
                }
                Err(burnchain_error::ParseError) => {
                    assert!(cfg.parse_error);
                }
                Err(burnchain_error::ThreadChannelError) => {
                    assert!(cfg.db_error || cfg.download_error || cfg.parse_error);
                }
                _ => panic!("Unexpected error: {:?}", res),
            }
        }
    }
}
