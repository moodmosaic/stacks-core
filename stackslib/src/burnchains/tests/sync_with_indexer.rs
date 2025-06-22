use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::burnchains::{Burnchain, BurnchainHeaderHash, CoordinatorChannels};
use crate::burnchains::tests::mock_indexer::{MockBurnchainIndexer, MockBlock};

#[test]
fn test_sync_with_indexer_happy_path() {
    // Arrange
    let first_block_hash = BurnchainHeaderHash::zero();
    let first_block_height = 0;
    let mut burnchain = Burnchain::default_unittest(first_block_height, &first_block_hash);
    
    // Create a mock indexer that provides headers and blocks
    let mut mock_indexer = MockBurnchainIndexer::new(
        vec![
            // Mock chain data - headers and blocks
            MockBlock::new(0, BurnchainHeaderHash::zero()),
            MockBlock::new(1, BurnchainHeaderHash::from_test_data(&[1])),
            MockBlock::new(2, BurnchainHeaderHash::from_test_data(&[2])),
        ],
        None, // No reorg
        None, // No download failure
    );
    
    let channels = CoordinatorChannels::new();
    let should_keep_running = Some(Arc::new(AtomicBool::new(true)));
    
    // Act
    let result = burnchain.sync_with_indexer(
        &mut mock_indexer, 
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
    
    // Check that db state matches expected heights
    assert_eq!(mock_indexer.get_db_height(), 2, "Expected mock indexer DB height to be 2");
}

#[test]
fn test_sync_with_indexer_download_failure() {
    // Arrange
    let first_block_hash = BurnchainHeaderHash::zero();
    let first_block_height = 0;
    let mut burnchain = Burnchain::default_unittest(first_block_height, &first_block_hash);
    
    // Create a mock indexer that will fail download at height 2
    let mut mock_indexer = MockBurnchainIndexer::new(
        vec![
            MockBlock::new(0, BurnchainHeaderHash::zero()),
            MockBlock::new(1, BurnchainHeaderHash::from_test_data(&[1])),
            MockBlock::new(2, BurnchainHeaderHash::from_test_data(&[2])),
        ],
        None,                   // No reorg
        Some(2),                // Fail download at height 2
    );
    
    let channels = CoordinatorChannels::new();
    let should_keep_running = Some(Arc::new(AtomicBool::new(true)));
    
    // Act
    let result = burnchain.sync_with_indexer(
        &mut mock_indexer, 
        channels,
        Some(2), // Target height
        None,    // Max blocks
        should_keep_running,
    );
    
    // Assert
    assert!(result.is_err(), "Expected sync to fail but it succeeded");
    
    // Check that DB heights are consistent (both should be at height 1)
    // Note: This assertion might need adjustment based on actual behavior
    assert_eq!(mock_indexer.get_db_height(), 1, "Expected mock indexer DB height to be 1 (before the failure point)");
}
