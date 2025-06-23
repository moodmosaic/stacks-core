use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool};

use crate::burnchains::Burnchain;
use crate::burnchains::Error as burnchain_error;
use crate::burnchains::bitcoin::{Error as bitcoin_error};
use crate::chainstate::coordinator::comm::CoordinatorCommunication;

use crate::burnchains::tests::test_doubles::{StubBlock, MockDownloader, MockBlockParser, MockIndexer, BurnchainIndexerTestDouble, TestHeaderIPC, TestBlockIPC};

use crate::burnchains::BurnchainBlockHeader;
use crate::burnchains::BurnchainBlock;

use stacks_common::types::chainstate::{BurnchainHeaderHash, TrieHash};

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
        StubBlock::new(1, BurnchainHeaderHash::from_test_data(1, &TrieHash::from_empty_data(), 0)),
        StubBlock::new(2, BurnchainHeaderHash::from_test_data(2, &TrieHash::from_empty_data(), 0)),
    ];
    
    // Create headers from blocks
    let headers: Vec<BurnchainBlockHeader> = blocks.iter().map(|b| b.to_header()).collect();
    
    // Create a mock indexer using mockall
    let mut test_double = BurnchainIndexerTestDouble::with_components(
        MockIndexer::<MockBlockParser<MockDownloader<TestHeaderIPC, TestBlockIPC>>>::new(),
    );
    
    // Set up mock downloader
    let mut mock_downloader = MockDownloader::<TestHeaderIPC, TestBlockIPC>::new();
    mock_downloader
        .expect_download()
        .return_once(|_header| Ok(TestBlockIPC {
            header: TestHeaderIPC { height: 1, hash: [0; 32] },
            data: vec![]
        }));
    
    // Set up mock block parser
    let mut mock_parser = MockBlockParser::new();
    mock_parser
        .expect_parse()
        .return_once(|_block, _epoch_id| {
            let stub = StubBlock::new(1, BurnchainHeaderHash::from_hex("0000000000000000000000000000000000000001").unwrap());
            Ok(BurnchainBlock::Bitcoin(stub.to_block()))
        });
    
    // Configure indexer.get_downloader() to return our mock
    test_double.indexer
        .expect_downloader()
        .return_once(move || mock_downloader);
    
    // Configure indexer.get_block_parser() to return our mock
    test_double.indexer
        .expect_parser()
        .return_once(move || mock_parser);
    
    // Keep track of which blocks were processed
    let processed_blocks = Arc::new(Mutex::new(Vec::new()));
    let processed_blocks_clone = processed_blocks.clone();
    
    // Configure process_block to record which blocks are processed
    // This is equivalent to checking DB height in the original tests
    test_double.indexer
        .expect_process_block()
        .times(3) // Expect exactly 3 calls (blocks 0, 1, 2)
        .returning(move |block_data| {
            let mut blocks = processed_blocks_clone.lock().unwrap();
            blocks.push(block_data.header.block_height);
            Ok(())
        });
        
    // Configure reader to return test double itself
    let test_double_clone = test_double.clone();
    test_double.indexer
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
    assert!(result.is_ok(), "Expected successful sync but got error: {:?}", result.err());
    
    if let Ok(header) = result {
        assert_eq!(header.block_height, 2, "Expected header height to be 2, got {}", header.block_height);
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
        StubBlock::new(1, BurnchainHeaderHash::from_test_data(1, &TrieHash::from_empty_data(), 0)),
        StubBlock::new(2, BurnchainHeaderHash::from_test_data(2, &TrieHash::from_empty_data(), 0)),
    ];
    
    // Create headers from blocks
    let headers: Vec<BurnchainBlockHeader> = blocks.iter().map(|b| b.to_header()).collect();
    
    // Create a mock indexer using mockall
    let mut test_double = BurnchainIndexerTestDouble::with_components(
        MockIndexer::<MockBlockParser<MockDownloader<TestHeaderIPC, TestBlockIPC>>>::new(),
    );
    
    // Set up mock downloader that fails
    let mut mock_downloader = MockDownloader::new();
    mock_downloader
        .expect_download()
        .returning(|_sender| Err(burnchain_error::DownloadError(bitcoin_error::ConnectionError)));
    
    // Set up mock block parser (which should not be called due to failure)
    let mut mock_parser = MockBlockParser::new();
    
    // Configure indexer.get_downloader() to return our mock
    test_double.indexer
        .expect_downloader()
        .return_once(move || mock_downloader);
    
    // Configure indexer.get_block_parser() to return our mock
    test_double.indexer
        .expect_parser()
        .return_once(move || mock_parser);
    
    // We don't need to configure process_block as it shouldn't be called
    
    // Configure process_headers to return empty epoch list in case it's called
    let test_double_clone2 = test_double.clone();
    test_double.indexer
        .expect_reader()
        .returning(move || test_double_clone2.indexer.clone());
    
    let (_receivers, channels) = CoordinatorCommunication::instantiate();
    let should_keep_running = Some(Arc::new(AtomicBool::new(true)));
    
    // Track which blocks are processed - in failure case, we should not see block 2 processed
    let processed_blocks = Arc::new(Mutex::new(Vec::new()));
    let processed_blocks_clone = processed_blocks.clone();
    
    // We might see process_block called for lower height blocks prior to download failure
    // But we should never see it called more than 2 times (blocks 0 & 1)
    test_double.indexer
        .expect_process_block()
        .returning(move |block_data| {
            let mut blocks = processed_blocks_clone.lock().unwrap();
            blocks.push(block_data.header.block_height);
            Ok(())
        });
        
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
        assert!(matches!(e, burnchain_error::DownloadError(_)), "Expected DownloadError, got {:?}", e);
    }
    
    // Verify that blocks at height 2 never processed (due to download failure)
    // This is equivalent to checking DB height in the original test
    let processed = processed_blocks.lock().unwrap();
    assert!(!processed.contains(&2), "Block at height 2 should not have been processed due to download failure");
    assert!(processed.len() <= 2, "Expected no more than 2 blocks to be processed");
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
        StubBlock::new(1, BurnchainHeaderHash::from_test_data(1, &TrieHash::from_empty_data(), 0)),
        StubBlock::new(2, BurnchainHeaderHash::from_test_data(2, &TrieHash::from_empty_data(), 0)),
    ];
    
    // Create headers from blocks
    let headers: Vec<BurnchainBlockHeader> = blocks.iter().map(|b| b.to_header()).collect();
    
    // Create a mock indexer using mockall
    let mut test_double = BurnchainIndexerTestDouble::with_components(
        MockIndexer::<MockBlockParser<MockDownloader<TestHeaderIPC, TestBlockIPC>>>::new(),
    );
    
    // Set up mock downloader based on test_config
    let mut mock_downloader = MockDownloader::new();
    if test_config.download_error {
        mock_downloader
            .expect_download()
            .returning(|_header| Err(burnchain_error::DownloadError(bitcoin_error::ConnectionError)));
    } else {
        mock_downloader
            .expect_download()
            .returning(|_header| {
                Ok(TestBlockIPC {
                    header: TestHeaderIPC { height: 1, hash: [0; 32] },
                    data: vec![]
                })
            });
    }
    
    // Set up mock block parser based on test_config
    let mut mock_parser = MockBlockParser::new();
    if test_config.parse_error {
        mock_parser
            .expect_parse()
            .returning(|_block, _epoch_id| Err(burnchain_error::ParseError));
    } else {
        mock_parser
            .expect_parse()
            .returning(|_block, _epoch_id| {
                let stub = StubBlock::new(1, BurnchainHeaderHash::from_hex("0000000000000000000000000000000000000001").unwrap());
                Ok(BurnchainBlock::Bitcoin(stub.to_block()))
            });
    }
    
    // Configure indexer.get_downloader() to return our mock
    test_double.indexer
        .expect_downloader()
        .return_once(move || mock_downloader);
    
    // Configure indexer.get_block_parser() to return our mock
    test_double.indexer
        .expect_parser()
        .return_once(move || mock_parser);
    
    // Configure reader to return test double itself
    let test_double_clone = test_double.clone();
    test_double.indexer
        .expect_reader()
        .returning(move || test_double_clone.indexer.clone());
    
    // Keep track of processed blocks to verify error scenarios work correctly
    let processed_blocks = Arc::new(Mutex::new(Vec::new()));
    let processed_blocks_clone = processed_blocks.clone();
    
    // Configure process_block expectations based on test_config
    if test_config.download_error {
        // With download error, process_block should never be called
        // mockall will automatically fail the test if it's called unexpectedly
        test_double.indexer
            .expect_process_block()
            .times(0) // Should never be called due to early download error
            .returning(|_| Ok(()));
    } else if test_config.parse_error {
        // With parse error, process_block should never be called
        test_double.indexer
            .expect_process_block()
            .times(0) // Should never be called due to early parse error
            .returning(|_| Ok(())); 
    } else if test_config.db_error {
        // With DB error, process_block gets called but returns error
        test_double.indexer
            .expect_process_block()
            .returning(move |block_data| {
                let mut blocks = processed_blocks_clone.lock().unwrap();
                blocks.push(block_data.header.block_height);
                Err(burnchain_error::DBError(crate::util_lib::db::Error::SqliteError(rusqlite::Error::ExecuteReturnedResults)))
            });
    } else {
        // No errors - process_block should succeed
        test_double.indexer
            .expect_process_block()
            .returning(move |block_data| {
                let mut blocks = processed_blocks_clone.lock().unwrap();
                blocks.push(block_data.header.block_height);
                Ok(())
            });
    }
    
    let (_receivers, channels) = CoordinatorCommunication::instantiate();
    let should_keep_running = Some(Arc::new(AtomicBool::new(true)));
    
    // Determine expected error type based on precedence
    let expected_error = if test_config.download_error {
        burnchain_error::DownloadError(bitcoin_error::ConnectionError)
    } else if test_config.parse_error {
        burnchain_error::ParseError
    } else if test_config.db_error {
        burnchain_error::DBError(crate::util_lib::db::Error::SqliteError(rusqlite::Error::ExecuteReturnedResults))
    } else {
        panic!("At least one error flag should be set");
    };
    
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
    
    match (result.err().unwrap(), expected_error) {
        (burnchain_error::DownloadError(_), burnchain_error::DownloadError(_)) => {},
        (burnchain_error::ParseError, burnchain_error::ParseError) => {},
        (burnchain_error::DBError(_), burnchain_error::DBError(_)) => {},
        (actual, expected) => {
            panic!("Expected {:?}, got {:?}", expected, actual);
        }
    }
}
