// =============================================================================
// layout.rs — レイアウト定数・GraphNode・Layout
// =============================================================================
//
// 変更が必要なケース:
//   - ノード間の余白・サイズ感を調整するとき
//   - フォントサイズを変更するとき（NODE_H の再計算も忘れずに）
//   - ズームの上下限・ステップを変えるとき
//   - GraphNode に座標ヘルパを追加するとき
// =============================================================================

use crate::types::ExpandDir;
use util::NodeRecord;

// ---------------------------------------------------------------------------
// ノード描画定数
// ---------------------------------------------------------------------------

pub const NODE_PADDING_X: f64 = 12.0;
pub const NODE_PADDING_Y: f64 = 8.0;
pub const LABEL_FONT_SIZE: f64 = 14.0;
pub const INFO_FONT_SIZE: f64 = 12.0;
pub const LABEL_LINE_H: f64 = 20.0;
pub const INFO_LINE_H: f64 = 18.0;
/// ノード高さ（固定）= padding * 2 + ラベル行 + 補足情報行
pub const NODE_H: f64 = NODE_PADDING_Y * 2.0 + LABEL_LINE_H + INFO_LINE_H;
pub const NODE_MIN_W: f64 = 160.0;
/// 全角1文字あたりの描画幅の目安
pub const FULL_ANGLE_CHAR_W: f64 = LABEL_FONT_SIZE;
pub const LABEL_MAX_CHARS: f64 = 14.0;
pub const LABEL_MAX_W: f64 = FULL_ANGLE_CHAR_W * LABEL_MAX_CHARS;
/// 記号+値 1件あたりの幅
pub const INFO_ITEM_W: f64 = 60.0;

// ---------------------------------------------------------------------------
// レイアウト定数
// ---------------------------------------------------------------------------

/// ノード間の水平余白
pub const H_GAP: f64 = 40.0;
/// ランク間の垂直余白
pub const V_GAP: f64 = 80.0;
/// ▲▼ハンドルの高さ領域
pub const HANDLE_H: f64 = 20.0;
/// ベジェ曲線の制御点オフセット
pub const EDGE_CTRL_DY: f64 = 40.0;

// ---------------------------------------------------------------------------
// ズーム定数
// ---------------------------------------------------------------------------

pub const ZOOM_MIN: f64 = 0.2;
pub const ZOOM_MAX: f64 = 3.0;
pub const ZOOM_STEP: f64 = 0.1;
/// この倍率未満のとき、ホバーしたノードを DOM ポップアップで等倍表示する
pub const POPUP_SCALE_THRESHOLD: f64 = 0.8;

// ---------------------------------------------------------------------------
// ヘッダー高さ（styles.rs の --header-h と合わせること）
// ---------------------------------------------------------------------------

pub const HEADER_H: f64 = 56.0;

// ---------------------------------------------------------------------------
// GraphNode — NodeRecord をラップした描画・レイアウト計算用の型
// ---------------------------------------------------------------------------

/// `NodeRecord`（シャードデータ）に Canvas 上の論理座標を付与した型。
/// 描画幅の計算・座標ヘルパ・ヒットテストをここに集約する。
/// `NodeRecord` を所有するため、キャッシュとは独立して使用できる。
pub struct GraphNode {
    /// シャードから読み込んだノードのデータ本体
    pub record: NodeRecord,
    /// ノード矩形の左上 x（論理座標系、pan/scale 適用前）
    pub x: f64,
    /// ノード矩形の左上 y（論理座標系、pan/scale 適用前）
    pub y: f64,
}

impl GraphNode {
    /// `NodeRecord` と配置座標からコンストラクト。
    pub fn new(record: NodeRecord, x: f64, y: f64) -> Self {
        Self { record, x, y }
    }

    // --- 描画幅 ---

    /// ノードの描画幅を計算する。
    /// ラベル幅と補足情報幅の大きい方に padding を加えた値。
    pub fn width(&self) -> f64 {
        let info_w = INFO_ITEM_W * self.record.info_item_count() as f64;
        (LABEL_MAX_W.max(info_w) + NODE_PADDING_X * 2.0).max(NODE_MIN_W)
    }

    // --- 座標ヘルパ ---

    pub fn cx(&self) -> f64 {
        self.x + self.width() / 2.0
    }
    pub fn cy(&self) -> f64 {
        self.y + NODE_H / 2.0
    }
    pub fn right(&self) -> f64 {
        self.x + self.width()
    }
    pub fn bottom(&self) -> f64 {
        self.y + NODE_H
    }

    /// ▲ハンドルの論理座標（ノード中央上）
    pub fn handle_up_center(&self) -> (f64, f64) {
        (self.cx(), self.y - HANDLE_H / 2.0)
    }

    /// ▼ハンドルの論理座標（ノード中央下）
    pub fn handle_down_center(&self) -> (f64, f64) {
        (self.cx(), self.bottom() + HANDLE_H / 2.0)
    }

    // --- ヒットテスト（論理座標） ---

    pub fn hit_body(&self, lx: f64, ly: f64) -> bool {
        lx >= self.x && lx <= self.right() && ly >= self.y && ly <= self.bottom()
    }

    pub fn hit_handle_up(&self, lx: f64, ly: f64) -> bool {
        let (hx, hy) = self.handle_up_center();
        let r = HANDLE_H;
        (lx - hx).abs() < r && (ly - hy).abs() < r / 2.0
    }

    pub fn hit_handle_down(&self, lx: f64, ly: f64) -> bool {
        let (hx, hy) = self.handle_down_center();
        let r = HANDLE_H;
        (lx - hx).abs() < r && (ly - hy).abs() < r / 2.0
    }
}

// ---------------------------------------------------------------------------
// Layout — 局所グラフのレイアウト計算結果
// ---------------------------------------------------------------------------

/// 起点 + 1ホップの局所グラフを表す。
/// `Layout::new` がコンストラクタを兼ねる。
#[derive(Default)]
pub struct Layout {
    /// 描画対象ノード（起点 + 先行群 + 後継群）
    pub nodes: Vec<GraphNode>,
    /// エッジ (from_id, to_id)
    pub edges: Vec<(u32, u32)>,
}

impl Layout {
    /// 起点ノードと隣接ノード群から局所グラフのレイアウトを構築する。
    ///
    /// # 座標系
    /// - 起点ノードの中央が論理座標 (0, 0)
    /// - 先行群は y 方向に上（負）、後継群は y 方向に下（正）
    /// - Canvas 描画時に `translate(canvas_w/2, canvas_h/2)` + pan + scale を適用
    ///
    /// # 引数
    /// - `root`  : 起点ノード
    /// - `preds` : 表示する先行ノード群（`ExpandDir::Down` のときは空）
    /// - `succs` : 表示する後継ノード群（`ExpandDir::Up` のときは空）
    pub fn new(root: NodeRecord, preds: Vec<NodeRecord>, succs: Vec<NodeRecord>) -> Self {
        let mut nodes: Vec<GraphNode> = Vec::new();
        let mut edges: Vec<(u32, u32)> = Vec::new();

        // ランク内の全ノードを横並びにしたときの合計幅を求める
        // （GraphNode を生成する前に幅だけ必要なため NodeRecord から計算）
        let rank_total_w = |records: &[NodeRecord]| -> f64 {
            if records.is_empty() {
                return 0.0;
            }
            let nodes_w: f64 = records
                .iter()
                .map(|r| {
                    let info_w = INFO_ITEM_W * r.info_item_count() as f64;
                    (LABEL_MAX_W.max(info_w) + NODE_PADDING_X * 2.0).max(NODE_MIN_W)
                })
                .sum();
            nodes_w + H_GAP * (records.len() as f64 - 1.0)
        };

        let root_w = {
            let info_w = INFO_ITEM_W * root.info_item_count() as f64;
            (LABEL_MAX_W.max(info_w) + NODE_PADDING_X * 2.0).max(NODE_MIN_W)
        };
        let pred_total_w = rank_total_w(&preds);
        let succ_total_w = rank_total_w(&succs);

        // 起点を (0, 0) に配置
        let root_y = 0.0_f64;
        let pred_y = root_y - NODE_H - V_GAP - HANDLE_H * 2.0;
        let succ_y = root_y + NODE_H + V_GAP + HANDLE_H * 2.0;
        let root_id = root.id;

        // 先行群: 中央揃えで横に配置
        if !preds.is_empty() {
            let mut px = -pred_total_w / 2.0;
            for pred in preds {
                let w = {
                    let info_w = INFO_ITEM_W * pred.info_item_count() as f64;
                    (LABEL_MAX_W.max(info_w) + NODE_PADDING_X * 2.0).max(NODE_MIN_W)
                };
                edges.push((pred.id, root_id));
                nodes.push(GraphNode::new(pred, px, pred_y));
                px += w + H_GAP;
            }
        }

        // 起点
        nodes.push(GraphNode::new(root, -root_w / 2.0, root_y));

        // 後継群: 中央揃えで横に配置
        if !succs.is_empty() {
            let mut sx = -succ_total_w / 2.0;
            for succ in succs {
                let w = {
                    let info_w = INFO_ITEM_W * succ.info_item_count() as f64;
                    (LABEL_MAX_W.max(info_w) + NODE_PADDING_X * 2.0).max(NODE_MIN_W)
                };
                edges.push((root_id, succ.id));
                nodes.push(GraphNode::new(succ, sx, succ_y));
                sx += w + H_GAP;
            }
        }

        Self { nodes, edges }
    }

    /// キャッシュ済みノード群と展開方向から `Layout` を構築するファクトリ。
    /// `App::rebuild_layout` から呼ばれる。
    pub fn from_cache(
        root: NodeRecord,
        expand_dir: ExpandDir,
        find: impl Fn(u32) -> Option<NodeRecord>,
    ) -> Self {
        let preds = match expand_dir {
            ExpandDir::Up | ExpandDir::Both => root
                .predecessors
                .iter()
                .filter_map(|&id| find(id))
                .collect(),
            ExpandDir::Down => vec![],
        };
        let succs = match expand_dir {
            ExpandDir::Down | ExpandDir::Both => {
                root.successors.iter().filter_map(|&id| find(id)).collect()
            }
            ExpandDir::Up => vec![],
        };
        Self::new(root, preds, succs)
    }
}
