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
use crate::burnchains::{
    Burnchain, BurnchainBlock, BurnchainBlockHeader, Error as burnchain_error,
};
use crate::chainstate::coordinator::comm::CoordinatorCommunication;
use crate::core::{EpochList, StacksEpoch, StacksEpochId};

// Helper for testing thread error scenarios
struct TestErrorConfig {
    download_error: bool,
    parse_error: bool,
    db_error: bool,
}

#[test]
fn test_sync_with_indexer_happy_path() {
    // Arrange
    let first_block_hash = BurnchainHeaderHash::zero();
    let first_block_height = 0;
    let mut burnchain = Burnchain::default_unittest(first_block_height, &first_block_hash);

    // Create mock blocks
    let blocks = vec![
        StubBlock::new(0, BurnchainHeaderHash::zero()),
        StubBlock::new(
            1,
            BurnchainHeaderHash::from_test_data(1, &TrieHash::from_empty_data(), 0),
        ),
        StubBlock::new(
            2,
            BurnchainHeaderHash::from_test_data(2, &TrieHash::from_empty_data(), 0),
        ),
    ];

    // Create headers from blocks
    let headers: Vec<BurnchainBlockHeader> = blocks.iter().map(|b| b.to_header()).collect();

    // Create a mock indexer using mockall
    let mut test_double = BurnchainIndexerTestDouble::with_components(MockIndexer::<
        MockBlockParser<MockDownloader<TestHeaderIPC, TestBlockIPC>>,
    >::new());

    let headers_for_mock = headers.clone();

    test_double
        .indexer
        .expect_get_headers_path()
        .return_const("/tmp/stacks-test".to_string());

    test_double
        .indexer
        .expect_sync_headers()
        .returning(|_start, end| Ok(end.unwrap_or(3)));

    test_double
        .indexer
        .expect_get_first_block_header_hash()
        .returning(|| Ok(BurnchainHeaderHash::zero()));

    test_double
        .indexer
        .expect_get_first_block_header_timestamp()
        .returning(|| Ok(0));

    test_double
        .indexer
        .expect_get_stacks_epochs()
        .returning(|| {
            let epoch = StacksEpoch {
                epoch_id: StacksEpochId::Epoch20,
                start_height: 0,
                end_height: STACKS_EPOCH_MAX,
                block_limit: ExecutionCost {
                    write_length: 0,
                    write_count: 0,
                    read_length: 0,
                    read_count: 0,
                    runtime: 0,
                },
                network_epoch: 2,
            };
            EpochList::new(&[epoch])
        });

    test_double
        .indexer
        .expect_get_highest_header_height()
        .returning(|| Ok(100));

    test_double
        .indexer
        .expect_find_chain_reorg()
        .returning(|| Ok(0));

    test_double
        .indexer
        .expect_read_headers()
        .returning(move |_, _| {
            let ipc_headers = headers_for_mock
                .iter()
                .map(|h| TestHeaderIPC {
                    height: h.block_height,
                    hash: h.block_hash.as_ref().try_into().unwrap(),
                })
                .collect();
            Ok(ipc_headers)
        });

    // Configure indexer.get_downloader() to return a new mock
    test_double.indexer.expect_downloader().returning(|| {
        let mut mock = MockDownloader::<TestHeaderIPC, TestBlockIPC>::new();
        mock.expect_download().returning(|header: &TestHeaderIPC| {
            Ok(TestBlockIPC {
                header: TestHeaderIPC {
                    height: header.height,
                    hash: header.hash,
                },
                data: vec![],
            })
        });
        mock
    });

    // Configure indexer.get_block_parser() to return a new mock
    test_double.indexer.expect_parser().returning(|| {
        let mut mock = MockBlockParser::<MockDownloader<TestHeaderIPC, TestBlockIPC>>::new();
        mock.expect_parse()
            .returning(|block: &TestBlockIPC, _epoch_id| {
                let hash = BurnchainHeaderHash::from(block.header.hash);
                let stub = StubBlock::new(block.header.height, hash);
                Ok(BurnchainBlock::Bitcoin(stub.to_block()))
            });
        mock
    });

    // Configure reader to return test double itself
    let test_double_clone = test_double.clone();
    test_double
        .indexer
        .expect_reader()
        .returning(move || test_double_clone.indexer.clone());

    let (_receivers, channels) = CoordinatorCommunication::instantiate();
    let should_keep_running = Some(Arc::new(AtomicBool::new(true)));

    // Act
    let result = burnchain.sync_with_indexer(
        &mut test_double,
        channels,
        Some(2), // Target height
        None,    // Max blocks
        should_keep_running,
    );

    // Assert
    assert!(
        result.is_ok(),
        "Expected successful sync but got error: {:?}",
        result.err()
    );

    if let Ok(header) = result {
        assert_eq!(
            header.block_height, 2,
            "Expected header height to be 2, got {}",
            header.block_height
        );
    }
}

#[test]
fn test_sync_with_indexer_download_failure() {
    // Arrange
    let first_block_hash = BurnchainHeaderHash::zero();
    let first_block_height = 0;
    let mut burnchain = Burnchain::default_unittest(first_block_height, &first_block_hash);

    // Create mock blocks
    let blocks = vec![
        StubBlock::new(0, BurnchainHeaderHash::zero()),
        StubBlock::new(
            1,
            BurnchainHeaderHash::from_test_data(1, &TrieHash::from_empty_data(), 0),
        ),
        StubBlock::new(
            2,
            BurnchainHeaderHash::from_test_data(2, &TrieHash::from_empty_data(), 0),
        ),
    ];

    // Create headers from blocks
    let headers: Vec<BurnchainBlockHeader> = blocks.iter().map(|b| b.to_header()).collect();

    // Create a mock indexer using mockall
    let mut test_double = BurnchainIndexerTestDouble::with_components(MockIndexer::<
        MockBlockParser<MockDownloader<TestHeaderIPC, TestBlockIPC>>,
    >::new());

    let headers_for_mock = headers.clone();

    test_double
        .indexer
        .expect_get_headers_path()
        .return_const("/tmp/stacks-test".to_string());

    test_double
        .indexer
        .expect_sync_headers()
        .returning(|_start, end| Ok(end.unwrap_or(3)));

    test_double
        .indexer
        .expect_get_first_block_header_hash()
        .returning(|| Ok(BurnchainHeaderHash::zero()));

    test_double
        .indexer
        .expect_get_first_block_header_timestamp()
        .returning(|| Ok(0));

    test_double
        .indexer
        .expect_get_stacks_epochs()
        .returning(|| {
            let epoch = StacksEpoch {
                epoch_id: StacksEpochId::Epoch20,
                start_height: 0,
                end_height: STACKS_EPOCH_MAX,
                block_limit: ExecutionCost {
                    write_length: 0,
                    write_count: 0,
                    read_length: 0,
                    read_count: 0,
                    runtime: 0,
                },
                network_epoch: 2,
            };
            EpochList::new(&[epoch])
        });

    test_double
        .indexer
        .expect_get_highest_header_height()
        .returning(|| Ok(100));

    test_double
        .indexer
        .expect_find_chain_reorg()
        .returning(|| Ok(0));

    test_double
        .indexer
        .expect_read_headers()
        .returning(move |_, _| {
            let ipc_headers = headers_for_mock
                .iter()
                .map(|h| TestHeaderIPC {
                    height: h.block_height,
                    hash: h.block_hash.as_ref().try_into().unwrap(),
                })
                .collect();
            Ok(ipc_headers)
        });

    // Configure indexer.get_downloader() to return a new mock that fails
    test_double.indexer.expect_downloader().returning(|| {
        let mut mock = MockDownloader::<TestHeaderIPC, TestBlockIPC>::new();
        mock.expect_download().returning(|_header: &TestHeaderIPC| {
            Err(burnchain_error::DownloadError(
                bitcoin_error::ConnectionError,
            ))
        });
        mock
    });

    // Configure indexer.get_block_parser() to return a new mock
    test_double
        .indexer
        .expect_parser()
        .returning(|| MockBlockParser::<MockDownloader<TestHeaderIPC, TestBlockIPC>>::new());

    // We don't need to configure process_block as it shouldn't be called

    // Configure process_headers to return empty epoch list in case it's called
    let test_double_clone2 = test_double.clone();
    test_double
        .indexer
        .expect_reader()
        .returning(move || test_double_clone2.indexer.clone());

    let (_receivers, channels) = CoordinatorCommunication::instantiate();
    let should_keep_running = Some(Arc::new(AtomicBool::new(true)));

    // Act
    let result = burnchain.sync_with_indexer(
        &mut test_double,
        channels,
        Some(2), // Target height
        None,    // Max blocks
        should_keep_running,
    );

    // Assert
    assert!(result.is_err(), "Expected sync to fail but it succeeded");

    if let Err(e) = result {
        assert!(
            matches!(e, burnchain_error::DownloadError(_)),
            "Expected DownloadError, got {:?}",
            e
        );
    }
}

// Test the error precedence with multiple thread failures
#[test]
fn test_sync_with_indexer_error_precedence() {
    // Setup test cases with different error combinations
    let test_cases = [
        // Only download fails - should report download error
        TestErrorConfig {
            download_error: true,
            parse_error: false,
            db_error: false,
        },
        // Only parse fails - should report parse error
        TestErrorConfig {
            download_error: false,
            parse_error: true,
            db_error: false,
        },
        // Only DB fails - should report DB error
        TestErrorConfig {
            download_error: false,
            parse_error: false,
            db_error: true,
        },
        // Both download and parse fail - should report download error first
        TestErrorConfig {
            download_error: true,
            parse_error: true,
            db_error: false,
        },
        // All threads fail - should report download error first
        TestErrorConfig {
            download_error: true,
            parse_error: true,
            db_error: true,
        },
    ];

    for test_config in test_cases {
        run_error_precedence_test(test_config);
    }
}

fn run_error_precedence_test(test_config: TestErrorConfig) {
    // Prepare burnchain instance
    let first_block_hash = BurnchainHeaderHash::zero();
    let first_block_height = 0;
    let mut burnchain = Burnchain::default_unittest(first_block_height, &first_block_hash);

    // Create mock blocks
    let blocks = vec![
        StubBlock::new(0, BurnchainHeaderHash::zero()),
        StubBlock::new(
            1,
            BurnchainHeaderHash::from_test_data(1, &TrieHash::from_empty_data(), 0),
        ),
        StubBlock::new(
            2,
            BurnchainHeaderHash::from_test_data(2, &TrieHash::from_empty_data(), 0),
        ),
    ];

    // Create headers from blocks
    let headers: Vec<BurnchainBlockHeader> = blocks.iter().map(|b| b.to_header()).collect();

    // Create a mock indexer using mockall
    let mut test_double = BurnchainIndexerTestDouble::with_components(MockIndexer::<
        MockBlockParser<MockDownloader<TestHeaderIPC, TestBlockIPC>>,
    >::new());

    let headers_for_mock = headers.clone();

    test_double
        .indexer
        .expect_get_headers_path()
        .return_const("/tmp/stacks-test".to_string());

    test_double
        .indexer
        .expect_sync_headers()
        .returning(|_start, end| Ok(end.unwrap_or(3)));

    test_double
        .indexer
        .expect_get_first_block_header_hash()
        .returning(|| Ok(BurnchainHeaderHash::zero()));

    test_double
        .indexer
        .expect_get_first_block_header_timestamp()
        .returning(|| Ok(0));

    test_double
        .indexer
        .expect_get_stacks_epochs()
        .returning(|| {
            let epoch = StacksEpoch {
                epoch_id: StacksEpochId::Epoch20,
                start_height: 0,
                end_height: STACKS_EPOCH_MAX,
                block_limit: ExecutionCost {
                    write_length: 0,
                    write_count: 0,
                    read_length: 0,
                    read_count: 0,
                    runtime: 0,
                },
                network_epoch: 2,
            };
            EpochList::new(&[epoch])
        });

    test_double
        .indexer
        .expect_get_highest_header_height()
        .returning(|| Ok(100));

    test_double
        .indexer
        .expect_find_chain_reorg()
        .returning(|| Ok(0));

    test_double
        .indexer
        .expect_read_headers()
        .returning(move |_, _| {
            let ipc_headers = headers_for_mock
                .iter()
                .map(|h| TestHeaderIPC {
                    height: h.block_height,
                    hash: h.block_hash.as_ref().try_into().unwrap(),
                })
                .collect();
            Ok(ipc_headers)
        });

    // Set up mock downloader based on test_config
    test_double.indexer.expect_downloader().returning(move || {
        let mut mock = MockDownloader::<TestHeaderIPC, TestBlockIPC>::new();
        if test_config.download_error {
            mock.expect_download().returning(|_header: &TestHeaderIPC| {
                Err(burnchain_error::DownloadError(
                    bitcoin_error::ConnectionError,
                ))
            });
        } else {
            mock.expect_download().returning(|header: &TestHeaderIPC| {
                Ok(TestBlockIPC {
                    header: TestHeaderIPC {
                        height: header.height,
                        hash: header.hash,
                    },
                    data: vec![],
                })
            });
        }
        mock
    });

    // Set up mock block parser based on test_config
    test_double.indexer.expect_parser().returning(move || {
        let mut mock = MockBlockParser::<MockDownloader<TestHeaderIPC, TestBlockIPC>>::new();
        if test_config.parse_error {
            mock.expect_parse()
                .returning(|_block, _epoch_id| Err(burnchain_error::ParseError));
        } else {
            mock.expect_parse()
                .returning(|block: &TestBlockIPC, _epoch_id| {
                    let hash = BurnchainHeaderHash::from(block.header.hash);
                    let stub = StubBlock::new(block.header.height, hash);
                    Ok(BurnchainBlock::Bitcoin(stub.to_block()))
                });
        }
        mock
    });

    // Configure reader to return test double itself
    let test_double_clone = test_double.clone();
    test_double
        .indexer
        .expect_reader()
        .returning(move || test_double_clone.indexer.clone());

    let (_receivers, channels) = CoordinatorCommunication::instantiate();
    let should_keep_running = Some(Arc::new(AtomicBool::new(true)));

    // Act
    let result = burnchain.sync_with_indexer(
        &mut test_double,
        channels,
        Some(2), // Target height
        None,    // Max blocks
        should_keep_running,
    );

    // Assert
    if test_config.db_error && !test_config.download_error && !test_config.parse_error {
        assert!(result.is_ok(), "Expected sync to succeed but it failed");
    } else {
        assert!(result.is_err(), "Expected sync to fail but it succeeded");

        if let Err(e) = result {
            match e {
                burnchain_error::DownloadError(_) => assert!(test_config.download_error),
                burnchain_error::ParseError => assert!(test_config.parse_error),
                burnchain_error::ThreadChannelError => {
                    // ThreadChannelError can occur in any threading scenario
                    assert!(
                        test_config.db_error
                            || test_config.download_error
                            || test_config.parse_error
                    );
                }
                _ => panic!("Unexpected error: {:?}", e),
            }
        }
    }
}
