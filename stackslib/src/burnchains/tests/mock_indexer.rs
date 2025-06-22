use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Sender, Receiver, sync_channel};

use crate::burnchains::{
    Burnchain,
    BurnchainBlock, 
    BurnchainBlockHeader, 
    BurnchainHeaderHash,
    BurnchainBlockData,
    BurnchainHeaderIPC,
    BurnchainBlockIPC,
    Indexer, 
    BlockParser, 
    Downloader, 
    BurnchainHeaderReader,
    burnchain_error,
    CoordinatorChannels,
};
use crate::chainstate::stacks::StacksEpochId;

/// Mock block for testing
pub struct MockBlock {
    height: u64,
    hash: BurnchainHeaderHash,
    parent_hash: BurnchainHeaderHash,
}

impl MockBlock {
    pub fn new(height: u64, hash: BurnchainHeaderHash) -> Self {
        // For simplicity, mock blocks at height 0 have parent hash of zero
        // Others use a simple pattern where parent hash is height-1
        let parent_hash = if height == 0 {
            BurnchainHeaderHash::zero()
        } else {
            BurnchainHeaderHash::from_test_data(&[(height-1) as u8])
        };
        
        Self { height, hash, parent_hash }
    }
    
    // Convert to header
    pub fn to_header(&self) -> BurnchainBlockHeader {
        BurnchainBlockHeader {
            block_height: self.height,
            block_hash: self.hash.clone(),
            parent_block_hash: self.parent_hash.clone(),
            num_txs: 0,
            timestamp: 0,
        }
    }
    
    // Convert to block
    pub fn to_block(&self) -> BurnchainBlock {
        BurnchainBlock {
            header: self.to_header(),
            txs: vec![],
        }
    }
}

/// Mock burnchain indexer for testing
pub struct MockBurnchainIndexer {
    // Mock chain state
    blocks: HashMap<u64, MockBlock>,
    current_height: u64,
    
    // Failure injection
    fail_download_at_height: Option<u64>,
    reorg_at_height: Option<u64>,
    
    // Internal state - tracks what has been processed
    db_height: u64,
}

impl MockBurnchainIndexer {
    pub fn new(
        blocks: Vec<MockBlock>, 
        reorg_at_height: Option<u64>,
        fail_download_at_height: Option<u64>,
    ) -> Self {
        let mut blocks_map = HashMap::new();
        let mut current_height = 0;
        
        // Convert block vector to map
        for block in blocks {
            current_height = current_height.max(block.height);
            blocks_map.insert(block.height, block);
        }
        
        Self {
            blocks: blocks_map,
            current_height,
            fail_download_at_height,
            reorg_at_height,
            db_height: 0, // Start at 0
        }
    }
    
    // Get the current db height (to check in tests)
    pub fn get_db_height(&self) -> u64 {
        self.db_height
    }
    
    // Internal helper to get block header
    fn get_header(&self, height: u64) -> Option<BurnchainBlockHeader> {
        self.blocks.get(&height).map(|b| b.to_header())
    }
}

impl BurnchainHeaderReader for MockBurnchainIndexer {
    fn read_burnchain_headers(
        &self,
        start_height: u64, 
        max_count: u64
    ) -> Result<Vec<BurnchainBlockHeader>, burnchain_error> {
        let mut headers = vec![];
        let end_height = start_height + max_count;
        
        for height in start_height..end_height {
            if let Some(block) = self.blocks.get(&height) {
                headers.push(block.to_header());
            } else {
                break;
            }
        }
        
        Ok(headers)
    }
    
    fn get_burnchain_headers_height(&self) -> Result<u64, burnchain_error> {
        Ok(self.current_height)
    }
}

// Mock downloader implementation
struct MockDownloader {
    blocks: Arc<HashMap<u64, MockBlock>>,
    fail_height: Option<u64>,
    receiver: Receiver<Option<BurnchainHeaderIPC>>,
}

impl Downloader for MockDownloader {
    fn download_blocks(&mut self, downloader_send: Sender<Option<BurnchainBlockIPC>>) -> Result<(), burnchain_error> {
        loop {
            match self.receiver.recv() {
                Ok(Some(header_ipc)) => {
                    let height = header_ipc.block_header.block_height;
                    
                    // Check if we should simulate a download failure
                    if let Some(fail_height) = self.fail_height {
                        if height == fail_height {
                            eprintln!("Simulating download failure at height {}", height);
                            return Err(burnchain_error::DownloadError);
                        }
                    }
                    
                    // Look up the block for this header
                    if let Some(block) = self.blocks.get(&height) {
                        let block_ipc = BurnchainBlockIPC {
                            block: block.to_block(),
                            block_header: header_ipc.block_header,
                        };
                        
                        if let Err(_) = downloader_send.send(Some(block_ipc)) {
                            return Err(burnchain_error::CoordinatorClosed);
                        }
                    }
                },
                Ok(None) => {
                    // End of headers
                    if let Err(_) = downloader_send.send(None) {
                        return Err(burnchain_error::CoordinatorClosed);
                    }
                    return Ok(());
                },
                Err(_) => {
                    return Err(burnchain_error::CoordinatorClosed);
                }
            }
        }
    }
}

// Mock block parser
struct MockBlockParser {}

impl BlockParser for MockBlockParser {
    fn parse_blocks(
        &mut self, 
        blocks_receiver: Receiver<Option<BurnchainBlockIPC>>,
        db_receiver: Sender<Option<BurnchainBlockData>>
    ) -> Result<(), burnchain_error> {
        loop {
            match blocks_receiver.recv() {
                Ok(Some(block_ipc)) => {
                    // Convert to block data - simple pass-through in the mock
                    let block_data = BurnchainBlockData {
                        header: block_ipc.block_header,
                        ops: vec![],
                    };
                    
                    if let Err(_) = db_receiver.send(Some(block_data)) {
                        return Err(burnchain_error::CoordinatorClosed);
                    }
                },
                Ok(None) => {
                    if let Err(_) = db_receiver.send(None) {
                        return Err(burnchain_error::CoordinatorClosed);
                    }
                    return Ok(());
                },
                Err(_) => {
                    return Err(burnchain_error::CoordinatorClosed);
                }
            }
        }
    }
}

impl Indexer for MockBurnchainIndexer {
    fn get_downloader(&self) -> Box<dyn Downloader> {
        // Create a pair of channels
        let (sender, receiver) = sync_channel(10);
        
        // Clone state for the downloader
        let blocks = Arc::new(self.blocks.clone());
        let fail_height = self.fail_download_at_height;
        
        Box::new(MockDownloader {
            blocks,
            fail_height,
            receiver,
        })
    }
    
    fn get_block_parser(&self) -> Box<dyn BlockParser> {
        Box::new(MockBlockParser {})
    }
    
    fn process_headers(
        &mut self,
        _burnchain: &Burnchain,
        header_data: BurnchainHeaderIPC,
        _ops: Vec<String>,
        _receipt_merkle_root: [u8; 32],
    ) -> Result<Vec<StacksEpochId>, burnchain_error> {
        // The mock doesn't need to do real processing
        Ok(vec![])
    }
    
    fn process_block(
        &mut self,
        _burnchain: &Burnchain,
        block_data: &BurnchainBlockData,
    ) -> Result<(), burnchain_error> {
        // Update our internal state for testing
        self.db_height = block_data.header.block_height;
        Ok(())
    }
}
