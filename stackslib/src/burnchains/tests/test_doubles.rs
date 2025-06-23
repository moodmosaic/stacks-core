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

use crate::burnchains::{BurnchainBlockHeader, BurnchainBlock};
use crate::burnchains::bitcoin::BitcoinBlock;
use crate::burnchains::indexer::{BurnchainBlockDownloader, BurnchainBlockParser, BurnchainIndexer};
use crate::burnchains::db::BurnchainHeaderReader;
use crate::burnchains::Error as burnchain_error;
use crate::burnchains::indexer::{BurnHeaderIPC, BurnBlockIPC};
use crate::burnchains::db::BurnchainBlockData;
use crate::core::{StacksEpochId};
use crate::core::EpochList;
use stacks_common::types::chainstate::BurnchainHeaderHash;
use crate::util_lib::db::Error as DBError;

// Simple Header IPC for testing
#[derive(Clone, Debug)]
pub struct TestHeaderIPC {
    pub height: u64,
    pub hash: [u8; 32],
}

impl BurnHeaderIPC for TestHeaderIPC {
    type H = TestHeaderIPC;
    
    fn height(&self) -> u64 {
        self.height
    }
    
    fn header(&self) -> Self::H {
        self.clone()
    }
    
    fn header_hash(&self) -> [u8; 32] {
        self.hash
    }
}

// Simple Block IPC for testing
#[derive(Clone, Debug)]
pub struct TestBlockIPC {
    pub header: TestHeaderIPC,
    pub data: Vec<u8>,
}

impl BurnBlockIPC for TestBlockIPC {
    type H = TestHeaderIPC;
    type B = TestBlockIPC;
    
    fn height(&self) -> u64 {
        self.header.height
    }
    
    fn header(&self) -> Self::H {
        self.header.clone()
    }
    
    fn block(&self) -> Self::B {
        self.clone()
    }
}

// Mock for BurnchainHeaderReader trait
mock! {
    pub MockBurnchainHeaderReader {}
    
    impl BurnchainHeaderReader for MockBurnchainHeaderReader {
        fn read_burnchain_headers(
            &self,
            start_height: u64, 
            end_height: u64
        ) -> Result<Vec<BurnchainBlockHeader>, DBError>;

        fn get_burnchain_headers_height(&self) -> Result<u64, DBError>;
        
        fn find_burnchain_header_height(
            &self,
            hash: &BurnchainHeaderHash
        ) -> Result<Option<u64>, DBError>;
    }
}

// Mock BurnchainBlockDownloader
mock! {
    pub Downloader<H = TestHeaderIPC, B = TestBlockIPC> {}
    impl<H, B> BurnchainBlockDownloader for Downloader<H, B>
    where
        H: BurnHeaderIPC + Sync + Send + Clone,
        B: BurnBlockIPC<H = H> + Sync + Send + Clone,
    {
        type H = H;
        type B = B;
        
        fn download(&mut self, header: &H) -> Result<B, burnchain_error>;
    }
}

// Mock BurnchainBlockParser
mock! {
    pub BlockParser<D: BurnchainBlockDownloader + Sync + Send = MockDownloader<TestHeaderIPC, TestBlockIPC>> {}
    impl<D> BurnchainBlockParser for BlockParser<D>
    where D: BurnchainBlockDownloader + Sync + Send
    {
        type D = D;
        
        fn parse(
            &mut self,
            block: &<<Self as BurnchainBlockParser>::D as BurnchainBlockDownloader>::B,
            epoch_id: StacksEpochId,
        ) -> Result<BurnchainBlock, burnchain_error>;
    }
}

// Mock BurnchainIndexer
mock! {
    pub Indexer<P: BurnchainBlockParser + Send + Sync> {
        pub fn process_block(&self, block_data: &BurnchainBlockData) -> Result<(), burnchain_error>;
    }
    impl<P: BurnchainBlockParser + Send + Sync> BurnchainIndexer for Indexer<P>
    {
        type P = P;
        
        fn connect(&mut self) -> Result<(), burnchain_error>;
        fn get_first_block_height(&self) -> u64;
        fn get_first_block_header_hash(&self) -> Result<BurnchainHeaderHash, burnchain_error>;
        fn get_first_block_header_timestamp(&self) -> Result<u64, burnchain_error>;
        fn get_stacks_epochs(&self) -> EpochList;
        fn get_headers_path(&self) -> String;
        fn get_headers_height(&self) -> Result<u64, burnchain_error>;
        fn get_highest_header_height(&self) -> Result<u64, burnchain_error>;
        fn find_chain_reorg(&mut self) -> Result<u64, burnchain_error>;
        fn sync_headers(
            &mut self,
            start_height: u64,
            end_height: Option<u64>,
        ) -> Result<u64, burnchain_error>;
        fn drop_headers(&mut self, new_height: u64) -> Result<(), burnchain_error>;
        fn read_headers(&self, start_block: u64, end_block: u64) -> Result<Vec<<P::D as BurnchainBlockDownloader>::H>, burnchain_error>;
        fn downloader(&self) -> <P as BurnchainBlockParser>::D;
        fn parser(&self) -> P;
        fn reader(&self) -> Self;
    }
}

// Manually implement Clone for MockIndexer
impl<P: BurnchainBlockParser + Send + Sync> Clone for MockIndexer<P> {
    fn clone(&self) -> Self {
        MockIndexer::new()
    }
}

// Test double for BurnchainHeaderReader and BurnchainIndexer traits.
// Satisfies sync_with_indexer()'s need for both in one object.
#[derive(Clone)]
pub struct BurnchainIndexerTestDouble<P = MockBlockParser<MockDownloader<TestHeaderIPC, TestBlockIPC>>>
where
    P: BurnchainBlockParser + Send + Sync + 'static,
{
    pub indexer: MockIndexer<P>,
}

impl<P> BurnchainHeaderReader for BurnchainIndexerTestDouble<P> 
where
    P: BurnchainBlockParser + Send + Sync 
{
    fn read_burnchain_headers(
        &self,
        start_height: u64, 
        end_height: u64
    ) -> Result<Vec<BurnchainBlockHeader>, DBError> {
        Err(DBError::NotImplemented)
    }
    
    fn get_burnchain_headers_height(&self) -> Result<u64, DBError> {
        Err(DBError::NotImplemented)
    }
    
    fn find_burnchain_header_height(
        &self,
        header_hash: &BurnchainHeaderHash,
    ) -> Result<Option<u64>, DBError> {
        Err(DBError::NotImplemented)
    }
}

impl<P> BurnchainIndexer for BurnchainIndexerTestDouble<P> 
where 
    P: BurnchainBlockParser + Send + Sync
{
    type P = P;
    
    fn connect(&mut self) -> Result<(), burnchain_error> {
        self.indexer.connect()
    }
    
    fn get_first_block_height(&self) -> u64 {
        self.indexer.get_first_block_height()
    }
    
    fn get_first_block_header_hash(&self) -> Result<BurnchainHeaderHash, burnchain_error> {
        self.indexer.get_first_block_header_hash()
    }
    
    fn get_first_block_header_timestamp(&self) -> Result<u64, burnchain_error> {
        self.indexer.get_first_block_header_timestamp()
    }
    
    fn get_stacks_epochs(&self) -> EpochList {
        self.indexer.get_stacks_epochs()
    }
    
    fn get_headers_path(&self) -> String {
        self.indexer.get_headers_path()
    }
    
    fn get_headers_height(&self) -> Result<u64, burnchain_error> {
        self.indexer.get_headers_height()
    }
    
    fn get_highest_header_height(&self) -> Result<u64, burnchain_error> {
        self.indexer.get_highest_header_height()
    }
    
    fn find_chain_reorg(&mut self) -> Result<u64, burnchain_error> {
        self.indexer.find_chain_reorg()
    }
    
    fn sync_headers(
        &mut self,
        start_height: u64,
        end_height: Option<u64>,
    ) -> Result<u64, burnchain_error> {
        self.indexer.sync_headers(start_height, end_height)
    }
    
    fn drop_headers(&mut self, new_height: u64) -> Result<(), burnchain_error> {
        self.indexer.drop_headers(new_height)
    }
    
    fn read_headers(&self, start_block: u64, end_block: u64) -> Result<Vec<<P::D as BurnchainBlockDownloader>::H>, burnchain_error> {
        self.indexer.read_headers(start_block, end_block)
    }
    
    fn downloader(&self) -> <P as BurnchainBlockParser>::D {
        self.indexer.downloader()
    }
    
    fn parser(&self) -> P {
        self.indexer.parser()
    }
    
    fn reader(&self) -> Self {
        self.clone()
    }
}

impl<P: BurnchainBlockParser + Send + Sync + 'static> BurnchainIndexerTestDouble<P> {
    pub fn with_components(indexer: MockIndexer<P>) -> Self {
        BurnchainIndexerTestDouble {
            indexer,
        }
    }
    
    /// Forward clone to components as both implement Clone
    pub fn clone(&self) -> Self {
        BurnchainIndexerTestDouble {
            indexer: self.indexer.clone(),
        }
    }
    
    /// Process block method for tests - not part of BurnchainIndexer trait
    pub fn process_block(&mut self, block: &BurnchainBlockData) -> Result<Vec<String>, burnchain_error> {
        // Test implementation that always succeeds
        Ok(vec![])
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
            BurnchainHeaderHash::from_hex(&format!("{:064x}", height - 1)).unwrap()
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
    
    pub fn to_block_data(&self) -> BurnchainBlockData {
        BurnchainBlockData {
            header: self.to_header(),
            ops: Vec::new(),
        }
    }
    
    // Returns BitcoinBlock for use in BurnchainBlock::Bitcoin variant
    pub fn to_block(&self) -> BitcoinBlock {
        // Create a minimal BitcoinBlock from the stub data
        BitcoinBlock {
            block_height: self.height,
            block_hash: self.hash.clone(),
            parent_block_hash: self.parent_hash.clone(),
            txs: vec![],
            timestamp: 0,
        }
    }
}
