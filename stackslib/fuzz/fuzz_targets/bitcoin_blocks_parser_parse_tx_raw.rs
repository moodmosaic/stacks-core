#![no_main]

use blockstack_lib::burnchains::bitcoin::blocks::BitcoinBlockParser;
use blockstack_lib::burnchains::bitcoin::BitcoinNetworkType;
use blockstack_lib::burnchains::MagicBytes;
use blockstack_lib::core::StacksEpochId;
use libfuzzer_sys::fuzz_target;
use stacks_common::deps_common::bitcoin::blockdata::transaction::Transaction;
use stacks_common::deps_common::bitcoin::network::serialize::deserialize;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() || data.len() < 8 {
        return;
    }

    if let Ok(tx) = deserialize::<Transaction>(&data) {
        if tx.output.len() == 0 {
            return;
        }

        let magic_bytes = MagicBytes::default();
        let parser = BitcoinBlockParser::new(BitcoinNetworkType::Mainnet, magic_bytes);
        let _ = parser.parse_tx(&tx, 0, StacksEpochId::Epoch31);
    }
});
