use std::collections::BTreeMap;

use itertools::Itertools;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Archive, Serialize, Deserialize)]
pub struct Node {
    pub from: Vec<u32>,
    pub to: Vec<u32>,
    pub label: String,
    pub data: NodeScore,
}

impl Node {
    pub fn new(from: Vec<u32>, to: Vec<u32>, label: String, data: NodeScore) -> Self {
        Self {
            from,
            to,
            label,
            data,
        }
    }
}

#[derive(Clone, Debug, Archive, Serialize, Deserialize, PartialEq)]
pub struct NodeScore {
    pub triple: u32,
    pub sequence: u32,
    pub neighbor: u32,
    pub skip: u32,
    pub double: u32,
}

impl NodeScore {
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

#[derive(Clone, Debug, PartialEq, Archive, Serialize, Deserialize)]
pub struct NodeRecord {
    pub id: u32,
    pub label: String,
    pub predecessors: Vec<u32>,
    pub successors: Vec<u32>,
    tiles: Vec<u32>,
    score: NodeScore,
}

impl NodeRecord {
    pub fn new(
        id: u32,
        tiles: Vec<u32>,
        score: NodeScore,
        predecessors: Vec<u32>,
        successors: Vec<u32>,
    ) -> Self {
        Self {
            id,
            label: tiles.iter().join(","),
            tiles,
            score,
            predecessors,
            successors,
        }
    }
}

pub const SHARD_SIZE: u32 = 15_625;
pub const TOTAL_NODES: u32 = 405_348;

use std::cmp::Ordering;

// ---------------------------------------------------------------------------
// NodeData trait
// ---------------------------------------------------------------------------

/// ノードデータへのアクセスインターフェース。
/// dag-viewer は NodeRecord の内部構造を知らずにこの trait 経由で操作する。
///
/// # 実装者へ
/// - `values()` の返す順序は `value_labels()` と対応させること
/// - `tier()` は自然数（0以上）を返すこと
/// - `sort_key()` の実装を変更することでソート規則を自由に変更できる
pub trait NodeData: Clone + 'static {
    /// 補足情報の値を順番に返す。
    /// `value_labels()` の i 番目のラベルが values() の i 番目の値に対応する。
    fn values(&self) -> impl Iterator<Item = u32> + '_;

    /// ノードの階層値（自然数）。
    /// 水平位置の決定に使用される。
    fn tier(&self) -> u32;

    /// ソートに使用するキーを返す。
    /// 戻り値の型は実装ごとに異なってよい（`impl Ord`）。
    /// dag-viewer は `compare_nodes()` 経由でのみ比較を行う。
    fn sort_key(&self) -> impl Ord + '_;
}

// ---------------------------------------------------------------------------
// ソート比較関数
// ---------------------------------------------------------------------------

/// 2つのノードを `sort_key()` に基づいて比較する。
///
/// dag-viewer はこの関数を `slice::sort_by` に渡してソートする。
/// ソート規則の変更は `NodeData::sort_key()` の実装変更のみで完結する。
///
/// # 例
/// ```rust
/// nodes.sort_by(|a, b| util::compare_nodes(a, b));
/// ```
pub fn compare_nodes<N: NodeData>(a: &N, b: &N) -> Ordering {
    a.sort_key()
        .partial_cmp(&b.sort_key())
        .unwrap_or(Ordering::Equal)
}

impl NodeData for NodeRecord {
    #[inline]
    fn tier(&self) -> u32 {
        self.tiles.len() as u32
    }

    /// ソートキーの実装。
    /// ここを変更することでソート規則を自由に変更できる。
    /// 現在は id 昇順。
    fn sort_key(&self) -> impl Ord + '_ {
        &self.tiles
    }

    fn values(&self) -> impl Iterator<Item = u32> + '_ {
        [
            self.score.triple,
            self.score.sequence,
            self.score.neighbor,
            self.score.skip,
            self.score.double,
        ]
        .into_iter()
    }
}

pub const VALUE_LABELS: &[&str] = &["triple", "sequence", "neighbor", "skip", "double"];

pub fn value_labels() -> &'static [&'static str] {
    &VALUE_LABELS
}
