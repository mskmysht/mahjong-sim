use std::collections::BTreeMap;

use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Archive, Serialize, Deserialize)]
pub struct Node {
    pub from: Vec<u32>,
    pub to: Vec<u32>,
    pub label: String,
    pub data: NodeData,
}

impl Node {
    pub fn new(from: Vec<u32>, to: Vec<u32>, label: String, data: NodeData) -> Self {
        Self {
            from,
            to,
            label,
            data,
        }
    }
}

#[derive(Clone, Debug, Archive, Serialize, Deserialize, PartialEq)]
pub struct NodeData {
    pub triple: u32,
    pub sequence: u32,
    pub neighbor: u32,
    pub skip: u32,
    pub double: u32,
}

impl NodeData {
    pub fn new(triple: u32, sequence: u32, neighbor: u32, skip: u32, double: u32) -> Self {
        Self {
            triple,
            sequence,
            neighbor,
            skip,
            double,
        }
    }

    fn text(&self) -> String {
        format!(
            "{} {} {} {} {}",
            self.sequence, self.triple, self.double, self.neighbor, self.skip,
        )
    }
}

#[derive(Debug, Archive, Serialize, Deserialize, Default)]
pub struct ShardMap {
    pub nodes: BTreeMap<u32, Node>,
}

pub const NUM_SHARD: u32 = 15625;

#[derive(Clone, Debug, PartialEq, Archive, Serialize, Deserialize)]
pub struct NodeRecord {
    pub id: u32,
    pub label: String,
    pub data: NodeData,
    // pub value_a: u32, // 仮名: 後で変更
    // pub value_b: u32, // 仮名: 後で変更
    // pub value_c: u32, // 仮名: 後で変更
    pub predecessors: Vec<u32>,
    pub successors: Vec<u32>,
}

pub const SHARD_SIZE: u32 = 15_625;
pub const TOTAL_NODES: u32 = 405_348;

impl NodeRecord {
    /// 補足情報の件数（記号+値の横一列アイテム数）。
    /// フィールドを追加した場合はここを変更すること。
    pub fn info_item_count(&self) -> usize {
        3 // value_a, value_b, value_c
    }

    /// 補足情報の表示文字列。
    /// フィールドを追加した場合はここを変更すること。
    pub fn info_text(&self) -> String {
        self.data.text()
    }
}
