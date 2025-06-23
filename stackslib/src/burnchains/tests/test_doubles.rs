// Copyright (C) 2025 Stacks Open Internet Foundation
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

// Burnchain sync test doubles.
// With respect to the xUnit Test Patterns book by Gerard Meszaros.
use mockall::predicate::*;
use mockall::*;
use stacks_common::types::chainstate::BurnchainHeaderHash;

use crate::burnchains::bitcoin::BitcoinBlock;
use crate::burnchains::db::{BurnchainBlockData, BurnchainHeaderReader};
use crate::burnchains::indexer::{
    BurnBlockIPC, BurnHeaderIPC, BurnchainBlockDownloader, BurnchainBlockParser, BurnchainIndexer,
};
use crate::burnchains::{BurnchainBlock, BurnchainBlockHeader, Error as burnchain_error};
use crate::core::{EpochList, StacksEpochId};
use crate::util_lib::db::Error as DBError;

// Stub block with canned values.
pub struct StubBlock {
    pub height: u64,
    pub hash: BurnchainHeaderHash,
    pub parent_hash: BurnchainHeaderHash,
}

impl StubBlock {
    pub fn new(height: u64, hash: BurnchainHeaderHash) -> Self {
        let parent = if height == 0 {
            BurnchainHeaderHash::zero()
        } else {
            let hex = format!("{:064x}", height - 1);
            BurnchainHeaderHash::from_hex(&hex).unwrap()
        };
        Self {
            height,
            hash,
            parent_hash: parent,
        }
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
            ops: vec![],
        }
    }

    pub fn to_block(&self) -> BitcoinBlock {
        BitcoinBlock {
            block_height: self.height,
            block_hash: self.hash.clone(),
            parent_block_hash: self.parent_hash.clone(),
            txs: vec![],
            timestamp: 0,
        }
    }
}

// Test header. Just height and hash.
#[derive(Clone, Debug)]
pub struct TestHeaderIPC {
    pub height: u64,
    pub hash: [u8; 32],
}

impl BurnHeaderIPC for TestHeaderIPC {
    type H = Self;

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

// Test block. Header plus bytes.
#[derive(Clone, Debug)]
pub struct TestBlockIPC {
    pub header: TestHeaderIPC,
    pub data: Vec<u8>,
}

impl BurnBlockIPC for TestBlockIPC {
    type H = TestHeaderIPC;
    type B = Self;

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

// Mock DB header reader.
mock! {
    pub MockBurnchainHeaderReader {}

    impl BurnchainHeaderReader for MockBurnchainHeaderReader {
        fn read_burnchain_headers(
            &self, start: u64, end: u64,
        ) -> Result<Vec<BurnchainBlockHeader>, DBError>;

        fn get_burnchain_headers_height(&self) -> Result<u64, DBError>;

        fn find_burnchain_header_height(
            &self, hash: &BurnchainHeaderHash,
        ) -> Result<Option<u64>, DBError>;
    }
}

// Mock block downloader.
mock! {
    pub Downloader<H = TestHeaderIPC, B = TestBlockIPC> {}

    impl<H, B> BurnchainBlockDownloader for Downloader<H, B>
    where
        H: BurnHeaderIPC + Sync + Send + Clone,
        B: BurnBlockIPC<H = H> + Sync + Send + Clone,
    {
        type H = H;
        type B = B;

        fn download(&mut self, header: &H)
            -> Result<B, burnchain_error>;
    }
}

// Mock block parser.
mock! {
    pub BlockParser<
        D: BurnchainBlockDownloader + Sync + Send =
            MockDownloader<TestHeaderIPC, TestBlockIPC>
    > {}

    impl<D> BurnchainBlockParser for BlockParser<D>
    where
        D: BurnchainBlockDownloader + Sync + Send,
    {
        type D = D;

        fn parse(
            &mut self,
            block: &<<Self as BurnchainBlockParser>::D
                as BurnchainBlockDownloader>::B,
            epoch_id: StacksEpochId,
        ) -> Result<BurnchainBlock, burnchain_error>;
    }
}

// Mock indexer. Implements trait and one extra method.
mock! {
    pub Indexer<P: BurnchainBlockParser + Send + Sync> {
        pub fn process_block(
            &self, block: &BurnchainBlockData,
        ) -> Result<(), burnchain_error>;
    }

    impl<P: BurnchainBlockParser + Send + Sync> BurnchainIndexer
        for Indexer<P>
    {
        type P = P;

        fn connect(&mut self) -> Result<(), burnchain_error>;
        fn get_first_block_height(&self) -> u64;
        fn get_first_block_header_hash(&self)
            -> Result<BurnchainHeaderHash, burnchain_error>;
        fn get_first_block_header_timestamp(&self)
            -> Result<u64, burnchain_error>;
        fn get_stacks_epochs(&self) -> EpochList;
        fn get_headers_path(&self) -> String;
        fn get_headers_height(&self) -> Result<u64, burnchain_error>;
        fn get_highest_header_height(&self)
            -> Result<u64, burnchain_error>;
        fn find_chain_reorg(&mut self) -> Result<u64, burnchain_error>;
        fn sync_headers(
            &mut self,
            start: u64,
            end: Option<u64>,
        ) -> Result<u64, burnchain_error>;
        fn drop_headers(
            &mut self, new_height: u64,
        ) -> Result<(), burnchain_error>;
        fn read_headers(
            &self,
            start: u64,
            end: u64,
        ) -> Result<
            Vec<<P::D as BurnchainBlockDownloader>::H>,
            burnchain_error,
        >;
        fn downloader(&self) -> <P as BurnchainBlockParser>::D;
        fn parser(&self) -> P;
        fn reader(&self) -> Self;
    }
}

impl<P: BurnchainBlockParser + Send + Sync> Clone for MockIndexer<P> {
    fn clone(&self) -> Self {
        MockIndexer::new()
    }
}

// Combines header reader and indexer traits.
#[derive(Clone)]
pub struct BurnchainIndexerTestDouble<
    P = MockBlockParser<MockDownloader<TestHeaderIPC, TestBlockIPC>>,
> where
    P: BurnchainBlockParser + Send + Sync + 'static,
{
    pub indexer: MockIndexer<P>,
}

impl<P> BurnchainHeaderReader for BurnchainIndexerTestDouble<P>
where
    P: BurnchainBlockParser + Send + Sync,
{
    fn read_burnchain_headers(
        &self,
        _start: u64,
        _end: u64,
    ) -> Result<Vec<BurnchainBlockHeader>, DBError> {
        Err(DBError::NotImplemented)
    }

    fn get_burnchain_headers_height(&self) -> Result<u64, DBError> {
        Err(DBError::NotImplemented)
    }

    fn find_burnchain_header_height(
        &self,
        _hash: &BurnchainHeaderHash,
    ) -> Result<Option<u64>, DBError> {
        Err(DBError::NotImplemented)
    }
}

impl<P> BurnchainIndexer for BurnchainIndexerTestDouble<P>
where
    P: BurnchainBlockParser + Send + Sync,
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

    fn sync_headers(&mut self, start: u64, end: Option<u64>) -> Result<u64, burnchain_error> {
        self.indexer.sync_headers(start, end)
    }

    fn drop_headers(&mut self, new_height: u64) -> Result<(), burnchain_error> {
        self.indexer.drop_headers(new_height)
    }

    fn read_headers(
        &self,
        start: u64,
        end: u64,
    ) -> Result<Vec<<P::D as BurnchainBlockDownloader>::H>, burnchain_error> {
        self.indexer.read_headers(start, end)
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

impl<P> BurnchainIndexerTestDouble<P>
where
    P: BurnchainBlockParser + Send + Sync + 'static,
{
    pub fn with_components(indexer: MockIndexer<P>) -> Self {
        Self { indexer }
    }

    pub fn clone(&self) -> Self {
        Self {
            indexer: self.indexer.clone(),
        }
    }

    pub fn process_block(
        &mut self,
        _block: &BurnchainBlockData,
    ) -> Result<Vec<String>, burnchain_error> {
        Ok(vec![])
    }
}
