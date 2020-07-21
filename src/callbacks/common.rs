use std::collections::HashMap;

use crate::blockchain::proto::tx::Tx;
use crate::blockchain::proto::tx::TxOutpoint;
use crate::blockchain::proto::Hashed;
use crate::blockchain::proto::ToRaw;
use crate::common::utils;

pub struct UnspentValue {
    pub block_height: u64,
    pub value: u64,
    pub address: String,
}

/// Iterates over transaction inputs and removes spent outputs from HashMap.
/// Returns the total number of processed inputs.
pub fn remove_unspents(tx: &Hashed<Tx>, unspents: &mut HashMap<Vec<u8>, UnspentValue>) -> u64 {
    for input in &tx.value.inputs {
        let key = input.outpoint.to_bytes();
        if unspents.contains_key(&key) {
            unspents.remove(&key);
        }
    }
    tx.value.in_count.value
}

/// Iterates over transaction outputs and adds valid unspents to HashMap.
/// Returns the total number of valid outputs.
pub fn insert_unspents(
    tx: &Hashed<Tx>,
    block_height: u64,
    unspents: &mut HashMap<Vec<u8>, UnspentValue>,
) -> u64 {
    let mut count = 0;
    for (i, output) in tx.value.outputs.iter().enumerate() {
        match &output.script.address {
            Some(address) => {
                let unspent = UnspentValue {
                    block_height,
                    address: address.clone(),
                    value: output.out.value,
                };

                let key = TxOutpoint::new(tx.hash, i as u32).to_bytes();
                unspents.insert(key, unspent);
                count += 1;
            }
            None => {
                debug!(
                    target: "callback", "Ignoring invalid utxo in: {} ({})",
                    utils::arr_to_hex_swapped(&tx.hash),
                    output.script.pattern
                );
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::parser::reader::BlockchainRead;
    use crate::blockchain::proto::header::BlockHeader;
    use crate::blockchain::proto::varuint::VarUint;
    use blockchain::proto::block::Block;
    use std::io::{BufReader, Cursor};

    #[test]
    fn test_callback() {
        let mut unspents: HashMap<Vec<u8>, UnspentValue> = HashMap::new();
        let header = BlockHeader {
            version: 0,
            prev_hash: [0u8; 32],
            merkle_root: [0u8; 32],
            timestamp: 0,
            bits: 0,
            nonce: 0,
        };

        // Create a mock of txid fff2525b8931402dd09222c50775608f75787bd2b87e56995a7bdd30f79702c4,
        // which increases balance of address 1JqDybm2nWTENrHvMyafbSXXtTk5Uv5QAn by 5.56 BTC.
        let raw_data = vec![
            0x01, 0x00, 0x00, 0x00, 0x01, 0x03, 0x2e, 0x38, 0xe9, 0xc0, 0xa8, 0x4c, 0x60, 0x46,
            0xd6, 0x87, 0xd1, 0x05, 0x56, 0xdc, 0xac, 0xc4, 0x1d, 0x27, 0x5e, 0xc5, 0x5f, 0xc0,
            0x07, 0x79, 0xac, 0x88, 0xfd, 0xf3, 0x57, 0xa1, 0x87, 0x00, 0x00, 0x00, 0x00, 0x8c,
            0x49, 0x30, 0x46, 0x02, 0x21, 0x00, 0xc3, 0x52, 0xd3, 0xdd, 0x99, 0x3a, 0x98, 0x1b,
            0xeb, 0xa4, 0xa6, 0x3a, 0xd1, 0x5c, 0x20, 0x92, 0x75, 0xca, 0x94, 0x70, 0xab, 0xfc,
            0xd5, 0x7d, 0xa9, 0x3b, 0x58, 0xe4, 0xeb, 0x5d, 0xce, 0x82, 0x02, 0x21, 0x00, 0x84,
            0x07, 0x92, 0xbc, 0x1f, 0x45, 0x60, 0x62, 0x81, 0x9f, 0x15, 0xd3, 0x3e, 0xe7, 0x05,
            0x5c, 0xf7, 0xb5, 0xee, 0x1a, 0xf1, 0xeb, 0xcc, 0x60, 0x28, 0xd9, 0xcd, 0xb1, 0xc3,
            0xaf, 0x77, 0x48, 0x01, 0x41, 0x04, 0xf4, 0x6d, 0xb5, 0xe9, 0xd6, 0x1a, 0x9d, 0xc2,
            0x7b, 0x8d, 0x64, 0xad, 0x23, 0xe7, 0x38, 0x3a, 0x4e, 0x6c, 0xa1, 0x64, 0x59, 0x3c,
            0x25, 0x27, 0xc0, 0x38, 0xc0, 0x85, 0x7e, 0xb6, 0x7e, 0xe8, 0xe8, 0x25, 0xdc, 0xa6,
            0x50, 0x46, 0xb8, 0x2c, 0x93, 0x31, 0x58, 0x6c, 0x82, 0xe0, 0xfd, 0x1f, 0x63, 0x3f,
            0x25, 0xf8, 0x7c, 0x16, 0x1b, 0xc6, 0xf8, 0xa6, 0x30, 0x12, 0x1d, 0xf2, 0xb3, 0xd3,
            0xff, 0xff, 0xff, 0xff, 0x02, 0x00, 0xe3, 0x23, 0x21, 0x00, 0x00, 0x00, 0x00, 0x19,
            0x76, 0xa9, 0x14, 0xc3, 0x98, 0xef, 0xa9, 0xc3, 0x92, 0xba, 0x60, 0x13, 0xc5, 0xe0,
            0x4e, 0xe7, 0x29, 0x75, 0x5e, 0xf7, 0xf5, 0x8b, 0x32, 0x88, 0xac, 0x00, 0x0f, 0xe2,
            0x08, 0x01, 0x00, 0x00, 0x00, 0x19, 0x76, 0xa9, 0x14, 0x94, 0x8c, 0x76, 0x5a, 0x69,
            0x14, 0xd4, 0x3f, 0x2a, 0x7a, 0xc1, 0x77, 0xda, 0x2c, 0x2f, 0x6b, 0x52, 0xde, 0x3d,
            0x7c, 0x88, 0xac, 0x00, 0x00, 0x00, 0x00,
        ];
        let mut reader = BufReader::new(Cursor::new(raw_data));
        let txs = reader.read_txs(1, 0x00).unwrap();
        let block1 = Block::new(0, header.clone(), VarUint::from(1u8), txs.clone());

        for tx in &block1.txs {
            remove_unspents(&tx, &mut unspents);
            insert_unspents(&tx, 100000, &mut unspents);
        }
        let value = unspents
            .get(&TxOutpoint::new(block1.txs[0].hash, 0).to_bytes())
            .unwrap();
        assert_eq!(value.block_height, 100000);
        assert_eq!(value.value, 556000000);
        assert_eq!(value.address, "1JqDybm2nWTENrHvMyafbSXXtTk5Uv5QAn");

        // Create a mock of txid 5aa8e36f9423ee5fcf17c1d0d45d6988b8a5773eae8ad25d945bf34352040009,
        // which decreases balance of address 1JqDybm2nWTENrHvMyafbSXXtTk5Uv5QAn by 5.56 BTC.
        let raw_data = vec![
            0x01, 0x00, 0x00, 0x00, 0x09, 0x82, 0x33, 0xbe, 0xef, 0x0f, 0x3a, 0xf0, 0x85, 0x56,
            0x23, 0xee, 0xba, 0x09, 0xe9, 0x6c, 0xf0, 0x62, 0xe7, 0xaf, 0xaf, 0x5c, 0x5a, 0xf1,
            0x66, 0x8e, 0x35, 0xb6, 0x8d, 0x11, 0xca, 0x1d, 0x79, 0x01, 0x00, 0x00, 0x00, 0x8b,
            0x48, 0x30, 0x45, 0x02, 0x21, 0x00, 0xdc, 0xd4, 0x43, 0xf7, 0x0a, 0x1c, 0xa9, 0x24,
            0x6d, 0x12, 0x71, 0x84, 0x2d, 0x47, 0x25, 0xdb, 0x4b, 0x3f, 0x90, 0xd7, 0x26, 0x90,
            0x55, 0x5a, 0x54, 0x5b, 0xbe, 0xaf, 0x50, 0xf9, 0xbf, 0xd8, 0x02, 0x20, 0x35, 0xba,
            0x5c, 0x03, 0x38, 0x4b, 0xd9, 0x3c, 0x5b, 0x33, 0x54, 0x0f, 0xa8, 0x3b, 0xc5, 0xc4,
            0x60, 0x01, 0xf8, 0xe0, 0x5a, 0xb5, 0x3d, 0x32, 0x29, 0x97, 0x58, 0xfb, 0xaf, 0x1f,
            0xf2, 0x2d, 0x01, 0x41, 0x04, 0x48, 0x88, 0x31, 0x8b, 0x7c, 0x43, 0x16, 0x4f, 0x3b,
            0xb2, 0xde, 0x45, 0x99, 0xe7, 0xfe, 0x08, 0xb6, 0x0d, 0xa9, 0x85, 0xce, 0x7d, 0xe7,
            0xb9, 0xaf, 0x68, 0xe1, 0x40, 0xe4, 0x8f, 0x26, 0x53, 0x9c, 0x9d, 0xfc, 0x5d, 0xf3,
            0x7d, 0x14, 0x58, 0x6c, 0x08, 0x6a, 0xb4, 0x96, 0xa7, 0x4f, 0x06, 0x0f, 0xc3, 0xd5,
            0xe9, 0x41, 0xcb, 0xea, 0x2f, 0xad, 0x6c, 0x40, 0xa3, 0x19, 0x3b, 0xa5, 0xea, 0xff,
            0xff, 0xff, 0xff, 0x03, 0xbd, 0x76, 0xc6, 0x15, 0x7d, 0xa4, 0x8e, 0x47, 0xa4, 0x24,
            0x74, 0xa9, 0xeb, 0x01, 0xb5, 0x14, 0xf8, 0x5b, 0x8e, 0x0a, 0xbc, 0x01, 0x26, 0xc1,
            0x62, 0x3a, 0x66, 0x51, 0x52, 0x6b, 0x35, 0x01, 0x00, 0x00, 0x00, 0x8a, 0x47, 0x30,
            0x44, 0x02, 0x20, 0x7e, 0xac, 0xa0, 0x1f, 0xcc, 0xab, 0xdb, 0x82, 0x92, 0x11, 0x57,
            0x27, 0x8f, 0x74, 0x3b, 0x89, 0xfa, 0x9d, 0x53, 0x54, 0xd6, 0x27, 0xae, 0x65, 0xb1,
            0xf6, 0x0c, 0xb4, 0x5b, 0x51, 0xf3, 0x13, 0x02, 0x20, 0x03, 0x9f, 0x1a, 0xf9, 0x6b,
            0x26, 0xb4, 0x6e, 0xc7, 0xc2, 0x1a, 0xb4, 0x58, 0x3d, 0xca, 0xb3, 0x8b, 0x6a, 0x2d,
            0x9f, 0xc3, 0xb7, 0x9e, 0xff, 0x60, 0x00, 0x71, 0x76, 0x7e, 0x4c, 0x8d, 0x96, 0x01,
            0x41, 0x04, 0x27, 0x33, 0x27, 0x71, 0xd3, 0xd7, 0xda, 0x5e, 0x4f, 0xec, 0xa7, 0xcc,
            0x5d, 0xac, 0x71, 0x2d, 0xdf, 0x95, 0x37, 0x63, 0x79, 0x66, 0xd1, 0x61, 0xae, 0x1c,
            0xea, 0xd9, 0x9d, 0xff, 0xad, 0xad, 0x5d, 0x99, 0x4d, 0x0a, 0x9c, 0x2a, 0xda, 0x8a,
            0xe0, 0xca, 0x3c, 0xd1, 0x21, 0x50, 0xbb, 0xc9, 0xc4, 0xc8, 0x5e, 0xf2, 0xc9, 0x79,
            0x52, 0xdb, 0xa9, 0xdd, 0xa7, 0x6a, 0xaa, 0x03, 0xa5, 0x28, 0xff, 0xff, 0xff, 0xff,
            0x0e, 0x64, 0x1a, 0x89, 0x4c, 0xf1, 0x8e, 0x97, 0x4d, 0x55, 0x65, 0x4a, 0x9b, 0xe1,
            0xb3, 0x50, 0x22, 0xd0, 0x10, 0x96, 0x8d, 0xed, 0x76, 0x9f, 0x65, 0x7f, 0x12, 0xfc,
            0xa1, 0x67, 0x91, 0x9b, 0x00, 0x00, 0x00, 0x00, 0x8a, 0x47, 0x30, 0x44, 0x02, 0x20,
            0x27, 0x58, 0xc2, 0x22, 0x55, 0x01, 0xaf, 0x4c, 0x4f, 0xaf, 0xc0, 0xf6, 0xbc, 0x77,
            0x92, 0xaa, 0xa2, 0x5b, 0x45, 0x99, 0xe0, 0x01, 0x1b, 0xd2, 0x9d, 0x10, 0x47, 0x36,
            0xa9, 0xc5, 0x07, 0xf1, 0x02, 0x20, 0x08, 0x4f, 0x5c, 0x1b, 0xdf, 0xdc, 0xa0, 0x93,
            0x85, 0x62, 0xf2, 0x21, 0xaf, 0x93, 0xbd, 0x55, 0x51, 0x25, 0x7f, 0xcb, 0x41, 0xcf,
            0xe0, 0x63, 0xfd, 0xf5, 0x9e, 0xcd, 0x28, 0x6f, 0x07, 0x4b, 0x01, 0x41, 0x04, 0xf4,
            0x6d, 0xb5, 0xe9, 0xd6, 0x1a, 0x9d, 0xc2, 0x7b, 0x8d, 0x64, 0xad, 0x23, 0xe7, 0x38,
            0x3a, 0x4e, 0x6c, 0xa1, 0x64, 0x59, 0x3c, 0x25, 0x27, 0xc0, 0x38, 0xc0, 0x85, 0x7e,
            0xb6, 0x7e, 0xe8, 0xe8, 0x25, 0xdc, 0xa6, 0x50, 0x46, 0xb8, 0x2c, 0x93, 0x31, 0x58,
            0x6c, 0x82, 0xe0, 0xfd, 0x1f, 0x63, 0x3f, 0x25, 0xf8, 0x7c, 0x16, 0x1b, 0xc6, 0xf8,
            0xa6, 0x30, 0x12, 0x1d, 0xf2, 0xb3, 0xd3, 0xff, 0xff, 0xff, 0xff, 0x3e, 0xcb, 0xb1,
            0x09, 0x35, 0x8d, 0xdc, 0x26, 0xdf, 0x7d, 0x96, 0x75, 0x80, 0x78, 0xb1, 0x52, 0x3c,
            0x7a, 0x95, 0x87, 0x7d, 0x45, 0x29, 0x0c, 0x8f, 0xb1, 0xb2, 0xda, 0xd6, 0x95, 0xf3,
            0xbe, 0x01, 0x00, 0x00, 0x00, 0x8a, 0x47, 0x30, 0x44, 0x02, 0x20, 0x00, 0xd8, 0x48,
            0xd5, 0x9c, 0x30, 0xe9, 0x5e, 0xc7, 0x2b, 0xb6, 0x65, 0x65, 0xc3, 0x9d, 0xf6, 0xad,
            0x50, 0xb1, 0x36, 0xf2, 0x1f, 0xf1, 0x60, 0x72, 0x2c, 0x14, 0xe5, 0xfc, 0xf1, 0xb7,
            0xa9, 0x02, 0x20, 0x32, 0x4b, 0xc6, 0x71, 0x5e, 0xd7, 0x0a, 0x10, 0xcc, 0xb7, 0x93,
            0xfe, 0x97, 0xf3, 0x7f, 0x03, 0x5e, 0x53, 0x85, 0x77, 0x98, 0x08, 0x06, 0x80, 0x12,
            0x7c, 0xac, 0xf6, 0x7e, 0xa6, 0x32, 0x85, 0x01, 0x41, 0x04, 0xfb, 0xde, 0x61, 0xe0,
            0x99, 0x18, 0xca, 0x46, 0x13, 0x45, 0xc5, 0xbe, 0xd2, 0x38, 0x0f, 0x0d, 0x4c, 0x0c,
            0xc0, 0x21, 0x77, 0x46, 0x0b, 0xe6, 0xa5, 0x2e, 0x70, 0xb6, 0xaf, 0x0e, 0xbf, 0xbd,
            0xdb, 0xdf, 0xeb, 0x1a, 0x99, 0x86, 0x06, 0x55, 0x08, 0x40, 0x80, 0x06, 0x42, 0x75,
            0xa8, 0x38, 0x0a, 0xaf, 0x8d, 0x15, 0x51, 0xd1, 0x87, 0x30, 0x51, 0x6b, 0x97, 0x5a,
            0xf4, 0x7c, 0x6b, 0xb7, 0xff, 0xff, 0xff, 0xff, 0x9b, 0xcb, 0x05, 0x05, 0xc4, 0x29,
            0xac, 0xc0, 0xa3, 0xf4, 0x67, 0xf2, 0xa9, 0x8e, 0xf7, 0x42, 0x64, 0x1e, 0xcc, 0xd2,
            0xc1, 0x85, 0x19, 0xbc, 0x98, 0x85, 0xe2, 0xb4, 0x50, 0xd0, 0x98, 0xa8, 0x00, 0x00,
            0x00, 0x00, 0x8c, 0x49, 0x30, 0x46, 0x02, 0x21, 0x00, 0xf0, 0x43, 0xb7, 0xb3, 0xe1,
            0x9f, 0x01, 0x09, 0x5c, 0xb3, 0x15, 0x65, 0x7f, 0xe1, 0xbe, 0x9c, 0x29, 0x62, 0xa3,
            0xa1, 0xb4, 0x34, 0x17, 0x68, 0x2b, 0x48, 0x50, 0x8d, 0xd2, 0xc4, 0x55, 0xd6, 0x02,
            0x21, 0x00, 0xab, 0xf5, 0xcd, 0xe3, 0xb8, 0xae, 0xca, 0x86, 0x9e, 0x61, 0x3e, 0xb1,
            0xdd, 0x14, 0xe3, 0x62, 0x8e, 0x2f, 0x8a, 0x77, 0xa6, 0x51, 0x92, 0xda, 0x8b, 0x57,
            0xb8, 0xbe, 0x3a, 0xb1, 0x20, 0x83, 0x01, 0x41, 0x04, 0xdc, 0x71, 0xd7, 0xd5, 0x09,
            0x0a, 0xf3, 0x5d, 0x5e, 0xc7, 0x28, 0x5b, 0x42, 0x44, 0xba, 0xa6, 0x5e, 0x3d, 0x96,
            0xb2, 0x92, 0x33, 0x26, 0x35, 0x8c, 0x50, 0x9d, 0xf5, 0x06, 0x23, 0xbc, 0x94, 0x03,
            0xd0, 0xcb, 0x77, 0x04, 0x8b, 0x4e, 0x3b, 0x0c, 0x77, 0x48, 0x09, 0x67, 0x49, 0x13,
            0xa2, 0xeb, 0x30, 0x99, 0x39, 0xb9, 0xa8, 0x66, 0x94, 0x30, 0xfe, 0xc8, 0x4d, 0x18,
            0xdd, 0xfe, 0x71, 0xff, 0xff, 0xff, 0xff, 0xa3, 0xed, 0x30, 0xe4, 0x11, 0x5c, 0xbe,
            0x4c, 0x6b, 0xc2, 0x3f, 0xcb, 0xab, 0xbc, 0x2a, 0x3b, 0x06, 0xdc, 0xb6, 0x34, 0xa4,
            0xbb, 0xf2, 0x0b, 0xe0, 0xc4, 0xb3, 0x6f, 0x0b, 0x83, 0x29, 0xa5, 0x00, 0x00, 0x00,
            0x00, 0x8a, 0x47, 0x30, 0x44, 0x02, 0x20, 0x07, 0x6f, 0xcb, 0x83, 0xdf, 0xed, 0x0b,
            0xb2, 0xbe, 0xba, 0x4a, 0x45, 0x39, 0x77, 0x05, 0xe9, 0x78, 0x66, 0x81, 0xda, 0x2a,
            0x82, 0x5f, 0x5f, 0xf1, 0x87, 0x71, 0xd4, 0xc0, 0x50, 0x96, 0x15, 0x02, 0x20, 0x65,
            0xd1, 0xb5, 0xa4, 0x10, 0x99, 0xca, 0x2e, 0xcd, 0xd3, 0xc6, 0xfa, 0x4d, 0xca, 0xe4,
            0x8c, 0xf5, 0xd4, 0xb8, 0x00, 0x3c, 0x47, 0xfa, 0x9e, 0x16, 0x1a, 0x35, 0xd2, 0x25,
            0xb8, 0x5e, 0x6d, 0x01, 0x41, 0x04, 0x7e, 0x86, 0x8e, 0xef, 0xc8, 0xe2, 0x4f, 0xf8,
            0x9a, 0xf5, 0x01, 0x7d, 0xa1, 0xba, 0xf8, 0xfc, 0x52, 0x8c, 0x75, 0x66, 0xed, 0x20,
            0x26, 0xcc, 0x80, 0x24, 0x4b, 0xa7, 0x6a, 0x0a, 0xdb, 0xca, 0x50, 0xba, 0x4d, 0x2e,
            0x0e, 0xc4, 0x74, 0x4c, 0x4d, 0x55, 0xab, 0x6a, 0x3f, 0x44, 0x26, 0x57, 0xf9, 0xd0,
            0x98, 0x10, 0x99, 0xd2, 0xe4, 0xe3, 0x33, 0x9e, 0x21, 0x0c, 0x6e, 0xfe, 0xe6, 0x47,
            0xff, 0xff, 0xff, 0xff, 0xc4, 0x02, 0x97, 0xf7, 0x30, 0xdd, 0x7b, 0x5a, 0x99, 0x56,
            0x7e, 0xb8, 0xd2, 0x7b, 0x78, 0x75, 0x8f, 0x60, 0x75, 0x07, 0xc5, 0x22, 0x92, 0xd0,
            0x2d, 0x40, 0x31, 0x89, 0x5b, 0x52, 0xf2, 0xff, 0x00, 0x00, 0x00, 0x00, 0x8b, 0x48,
            0x30, 0x45, 0x02, 0x20, 0x2f, 0x3f, 0xa1, 0x41, 0x3d, 0x76, 0x9e, 0xee, 0x26, 0xc2,
            0xec, 0xef, 0x3f, 0x3e, 0xf8, 0x26, 0xb5, 0x2b, 0xc4, 0x0f, 0xca, 0xa1, 0x77, 0xfc,
            0xb6, 0x0a, 0x23, 0x8c, 0x24, 0xad, 0x30, 0x6a, 0x02, 0x21, 0x00, 0xa8, 0x2a, 0x2b,
            0xd5, 0x4f, 0x88, 0x74, 0xb4, 0x14, 0x2f, 0x76, 0xb1, 0x27, 0x18, 0x9a, 0x9b, 0xf4,
            0xd0, 0xc5, 0xf4, 0xc4, 0x3d, 0xbd, 0x71, 0xbb, 0xdc, 0xcd, 0xf5, 0x8f, 0x0e, 0x3f,
            0x9b, 0x01, 0x41, 0x04, 0xef, 0x70, 0x9b, 0x53, 0x79, 0x56, 0x7c, 0xe8, 0xb5, 0xb2,
            0xc4, 0xbd, 0x0e, 0xfd, 0x01, 0xff, 0x1b, 0x6f, 0x56, 0xdc, 0xd2, 0x13, 0x93, 0x7f,
            0x56, 0xac, 0x23, 0x70, 0x20, 0x26, 0x30, 0xa7, 0xd1, 0xfd, 0x50, 0x86, 0xb5, 0xe8,
            0x06, 0x09, 0x08, 0x57, 0xa0, 0xa0, 0x09, 0xb0, 0x8a, 0x87, 0xce, 0x28, 0x32, 0x74,
            0xd8, 0x17, 0x8d, 0x71, 0xb4, 0xf2, 0x71, 0x8d, 0x79, 0x06, 0x45, 0xeb, 0xff, 0xff,
            0xff, 0xff, 0xca, 0x50, 0x65, 0xff, 0x96, 0x17, 0xcb, 0xcb, 0xa4, 0x5e, 0xb2, 0x37,
            0x26, 0xdf, 0x64, 0x98, 0xa9, 0xb9, 0xca, 0xfe, 0xd4, 0xf5, 0x4c, 0xba, 0xb9, 0xd2,
            0x27, 0xb0, 0x03, 0x5d, 0xde, 0xfb, 0x01, 0x00, 0x00, 0x00, 0x8c, 0x49, 0x30, 0x46,
            0x02, 0x21, 0x00, 0xca, 0xbd, 0x73, 0x2a, 0xcf, 0x73, 0x06, 0xb9, 0x56, 0x5e, 0x67,
            0x61, 0x79, 0xb3, 0xd1, 0x44, 0xcc, 0x5a, 0xf5, 0xde, 0x2d, 0x06, 0x18, 0xd7, 0x00,
            0xba, 0x28, 0x63, 0xa5, 0x3d, 0xa6, 0x62, 0x02, 0x21, 0x00, 0xaa, 0x2c, 0xff, 0x8a,
            0x41, 0x64, 0x90, 0x4a, 0x6b, 0x1a, 0x6e, 0xf0, 0x27, 0x9f, 0x02, 0x2b, 0xc7, 0xa0,
            0x2d, 0xfa, 0x9a, 0x59, 0xb8, 0x8e, 0xf5, 0x0a, 0x87, 0xef, 0xdb, 0xf0, 0xf5, 0xef,
            0x01, 0x41, 0x04, 0x56, 0xd5, 0x34, 0x67, 0xbd, 0x7d, 0x2a, 0xfc, 0x5c, 0xa6, 0x00,
            0x3e, 0x51, 0x0d, 0xec, 0x95, 0xd5, 0x9d, 0x65, 0x8b, 0x9e, 0x3e, 0x8a, 0xf4, 0x95,
            0x0f, 0x17, 0x0f, 0x39, 0x2e, 0x8a, 0xaf, 0xbb, 0x83, 0x87, 0xbc, 0x1e, 0xba, 0x8e,
            0xa5, 0xd4, 0xe2, 0x7d, 0xad, 0x8a, 0x2c, 0x60, 0x39, 0x66, 0xf2, 0xe0, 0xe0, 0x61,
            0x8d, 0xd7, 0x88, 0x47, 0xb3, 0x9f, 0xd8, 0xcf, 0x7f, 0x81, 0xd5, 0xff, 0xff, 0xff,
            0xff, 0xf9, 0x8a, 0x52, 0x64, 0xd2, 0xdf, 0xe1, 0x81, 0xa9, 0xbc, 0xf1, 0xdd, 0x5b,
            0x80, 0xd2, 0x49, 0xde, 0x24, 0x02, 0x42, 0xb7, 0x94, 0x19, 0xa8, 0x5a, 0xb2, 0x0f,
            0xd5, 0x19, 0x0b, 0x8f, 0x1a, 0x00, 0x00, 0x00, 0x00, 0x8b, 0x48, 0x30, 0x45, 0x02,
            0x20, 0x14, 0x10, 0x75, 0x1a, 0xd9, 0xa8, 0xc5, 0x68, 0x98, 0x95, 0xfa, 0x88, 0x61,
            0x48, 0x17, 0x57, 0xce, 0xa3, 0x23, 0xb8, 0x31, 0x0e, 0x6c, 0xf1, 0x8e, 0xc8, 0xc9,
            0x0d, 0x2f, 0xeb, 0x6b, 0xfe, 0x02, 0x21, 0x00, 0xd9, 0x4e, 0x56, 0x7e, 0xbe, 0xf0,
            0x6f, 0xfb, 0x06, 0xc5, 0xad, 0x67, 0x8f, 0x50, 0x77, 0x8c, 0xd6, 0x87, 0x78, 0x0f,
            0xf7, 0xc3, 0xdf, 0x3f, 0xea, 0x17, 0x7b, 0x78, 0xe3, 0xf7, 0x62, 0x22, 0x01, 0x41,
            0x04, 0x16, 0x09, 0x78, 0x42, 0x69, 0xe4, 0x3d, 0xcc, 0x8b, 0xd9, 0x91, 0x8e, 0x06,
            0xb8, 0x68, 0xf5, 0xc1, 0xf1, 0x71, 0x40, 0x8a, 0xd2, 0x65, 0x43, 0x75, 0x3a, 0xad,
            0x9d, 0xc7, 0x79, 0x1c, 0x57, 0xaf, 0x9e, 0x0d, 0xa5, 0x6a, 0xbc, 0x6b, 0x3b, 0x52,
            0x8d, 0xb2, 0x77, 0x07, 0x60, 0xc9, 0xbd, 0x0c, 0x06, 0x66, 0x96, 0x20, 0x94, 0x54,
            0x46, 0x51, 0x5a, 0x98, 0xf8, 0x57, 0x3e, 0x7c, 0x07, 0xff, 0xff, 0xff, 0xff, 0x01,
            0x80, 0x37, 0x9d, 0x1c, 0x02, 0x00, 0x00, 0x00, 0x19, 0x76, 0xa9, 0x14, 0x94, 0x90,
            0x02, 0x3a, 0x1f, 0x27, 0xc8, 0xf0, 0x95, 0x6a, 0x96, 0x3f, 0x36, 0x5f, 0x72, 0x68,
            0x72, 0xdc, 0x35, 0x92, 0x88, 0xac, 0x00, 0x00, 0x00, 0x00,
        ];
        let mut reader = BufReader::new(Cursor::new(raw_data));
        let txs = reader.read_txs(1, 0x00).unwrap();
        let block2 = Block::new(0, header.clone(), VarUint::from(1u8), txs.clone());

        for tx in &block2.txs {
            remove_unspents(&tx, &mut unspents);
            insert_unspents(&tx, 105001, &mut unspents);
        }

        // Original unspent should no longer exist in the hashmap
        assert!(unspents
            .get(&TxOutpoint::new(block1.txs[0].hash, 0).to_bytes())
            .is_none());

        let value = unspents
            .get(&TxOutpoint::new(block2.txs[0].hash, 0).to_bytes())
            .unwrap();

        assert_eq!(value.block_height, 105001);
        assert_eq!(value.value, 9070000000);
        assert_eq!(value.address, "1EYXXHs5gV4pc7QAddmDj5z7m14QPHGvWL");
    }
}
