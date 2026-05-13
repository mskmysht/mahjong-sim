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

#[derive(Debug, Archive, Serialize, Deserialize)]
pub struct NodeData {
    triple: u32,
    sequence: u32,
    neighbor: u32,
    skip: u32,
    double: u32,
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
}

#[derive(Debug, Archive, Serialize, Deserialize, Default)]
pub struct ShardMap {
    pub nodes: BTreeMap<u32, Node>,
}

pub const NUM_SHARD: u32 = 15625;