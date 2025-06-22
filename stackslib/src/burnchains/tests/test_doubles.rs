//! Test Doubles for burnchain synchronization tests.
//!
//! Hierarchy of Test Doubles:
//!
//! ```text
//!                      +---------------+
//!                      | Test Double   |
//!                      +---------------+
//!                              |
//!                              |
//!       +--------+--------+--------+--------+--------+
//!       |        |        |        |        |        |
//!   +--------+ +--------+ +--------+ +--------+ +--------+
//!   | Dummy  | | Stub   | | Spy    | | Mock   | | Fake   |
//!   +--------+ +--------+ +--------+ +--------+ +--------+
//! ```
//!
//! Reference: *xUnit Test Patterns: Refactoring Test Code* by Gerard Meszaros.

use mockall::predicate::*;
use mockall::*;

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
};
use crate::chainstate::stacks::StacksEpochId;
use std::sync::mpsc::{Sender, Receiver};

mock! {
    pub BurnchainHeaderReader {}
    impl BurnchainHeaderReader for BurnchainHeaderReader {
        fn read_burnchain_headers(
            &self,
            start_height: u64, 
            max_count: u64
        ) -> Result<Vec<BurnchainBlockHeader>, burnchain_error>;
        
        fn get_burnchain_headers_height(&self) -> Result<u64, burnchain_error>;
    }
}

mock! {
    pub Downloader {}
    impl Downloader for Downloader {
        fn download_blocks(&mut self, downloader_send: Sender<Option<BurnchainBlockIPC>>) -> Result<(), burnchain_error>;
    }
}

mock! {
    pub BlockParser {}
    impl BlockParser for BlockParser {
        fn parse_blocks(
            &mut self, 
            blocks_receiver: Receiver<Option<BurnchainBlockIPC>>,
            db_receiver: Sender<Option<BurnchainBlockData>>
        ) -> Result<(), burnchain_error>;
    }
}

mock! {
    pub Indexer {}
    impl Indexer for Indexer {
        fn get_downloader(&self) -> Box<dyn Downloader>;
        
        fn get_block_parser(&self) -> Box<dyn BlockParser>;
        
        fn process_headers(
            &mut self,
            burnchain: &Burnchain,
            header_data: BurnchainHeaderIPC,
            ops: Vec<String>,
            receipt_merkle_root: [u8; 32],
        ) -> Result<Vec<StacksEpochId>, burnchain_error>;
        
        fn process_block(
            &mut self,
            burnchain: &Burnchain,
            block_data: &BurnchainBlockData,
        ) -> Result<(), burnchain_error>;
    }
}

// Test double for BurnchainHeaderReader and Indexer traits.
// Satisfies sync_with_indexer()'s need for both in one object.
pub struct BurnchainIndexerTestDouble {
    pub header_reader: MockBurnchainHeaderReader,
    pub indexer: MockIndexer,
}

impl BurnchainHeaderReader for BurnchainIndexerTestDouble {
    fn read_burnchain_headers(
        &self,
        start_height: u64, 
        max_count: u64
    ) -> Result<Vec<BurnchainBlockHeader>, burnchain_error> {
        self.header_reader.read_burnchain_headers(start_height, max_count)
    }
    
    fn get_burnchain_headers_height(&self) -> Result<u64, burnchain_error> {
        self.header_reader.get_burnchain_headers_height()
    }
}

impl Indexer for BurnchainIndexerTestDouble {
    fn get_downloader(&self) -> Box<dyn Downloader> {
        self.indexer.get_downloader()
    }
    
    fn get_block_parser(&self) -> Box<dyn BlockParser> {
        self.indexer.get_block_parser()
    }
    
    fn process_headers(
        &mut self,
        burnchain: &Burnchain,
        header_data: BurnchainHeaderIPC,
        ops: Vec<String>,
        receipt_merkle_root: [u8; 32],
    ) -> Result<Vec<StacksEpochId>, burnchain_error> {
        self.indexer.process_headers(burnchain, header_data, ops, receipt_merkle_root)
    }
    
    fn process_block(
        &mut self,
        burnchain: &Burnchain,
        block_data: &BurnchainBlockData,
    ) -> Result<(), burnchain_error> {
        self.indexer.process_block(burnchain, block_data)
    }
}

// Stub block used in tests. Supplies canned data. No behavior check.
pub struct StubBlock {
    pub height: u64,
    pub hash: BurnchainHeaderHash,
    pub parent_hash: BurnchainHeaderHash,
}

impl StubBlock {
    pub fn new(height: u64, hash: BurnchainHeaderHash) -> Self {
        // Height 0 blocks use zero as parent hash for simplicity.
        // Others use height - 1 as parent hash.
        let parent_hash = if height == 0 {
            BurnchainHeaderHash::zero()
        } else {
            BurnchainHeaderHash::from_hex(&format!("{:016x}", height - 1)).unwrap()
        };
        Self { height, hash, parent_hash }
    }
    
    pub fn to_header(&self) -> BurnchainBlockHeader {
        BurnchainBlockHeader {
            block_height: self.height,
            block_hash: self.hash.clone(),
            parent_block_hash: self.parent_hash.clone(),
            num_txs: 0,
            timestamp: 0,
        }
    }
    
    pub fn to_block(&self) -> BurnchainBlock {
        BurnchainBlock {
            header: self.to_header(),
            txs: Vec::new(),
        }
    }
}
