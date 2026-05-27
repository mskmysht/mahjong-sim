// =============================================================================
// layout.rs — レイアウト定数・GraphNode・Layout
// =============================================================================
//
// 変更が必要なケース:
//   - ノード間の余白・サイズ感を調整するとき
//   - フォントサイズを変更するとき（NODE_H の再計算も忘れずに）
//   - ズームの上下限・ステップを変えるとき
// =============================================================================

use std::collections::HashMap;

use util::NodeRecord;

// ---------------------------------------------------------------------------
// ノード描画定数
// ---------------------------------------------------------------------------

pub const NODE_PADDING_X:    f64 = 12.0;
pub const NODE_PADDING_Y:    f64 = 8.0;
pub const LABEL_FONT_SIZE:   f64 = 14.0;
pub const INFO_FONT_SIZE:    f64 = 12.0;
pub const LABEL_LINE_H:      f64 = 20.0;
pub const INFO_LINE_H:       f64 = 18.0;
pub const NODE_H:            f64 = NODE_PADDING_Y * 2.0 + LABEL_LINE_H + INFO_LINE_H;
pub const FULL_ANGLE_CHAR_W: f64 = LABEL_FONT_SIZE;
pub const LABEL_MAX_CHARS:   f64 = 14.0;
pub const LABEL_MAX_W:       f64 = FULL_ANGLE_CHAR_W * LABEL_MAX_CHARS;
pub const INFO_ITEM_W:       f64 = 60.0;
/// ノード幅（全角14文字基準で全列固定）
pub const NODE_W:            f64 = LABEL_MAX_W + NODE_PADDING_X * 2.0;

// ---------------------------------------------------------------------------
// レイアウト定数
// ---------------------------------------------------------------------------

/// 列間の水平余白（ハンドル領域を含む）
pub const H_GAP:        f64 = 60.0;
/// 同一列内のノード間の垂直余白
pub const V_GAP:        f64 = 12.0;
/// ◀▶ハンドルの幅領域
pub const HANDLE_W:     f64 = 20.0;
/// ベジェ曲線の制御点水平オフセット
pub const EDGE_CTRL_DX: f64 = 40.0;
/// ID テキストのフォントサイズ
pub const ID_FONT_SIZE: f64 = 10.0;

// ---------------------------------------------------------------------------
// ズーム定数
// ---------------------------------------------------------------------------

pub const ZOOM_MIN:              f64 = 0.2;
pub const ZOOM_MAX:              f64 = 3.0;
pub const ZOOM_STEP:             f64 = 0.1;
pub const POPUP_SCALE_THRESHOLD: f64 = 0.8;

// ---------------------------------------------------------------------------
// ヘッダー高さ
// ---------------------------------------------------------------------------

pub const HEADER_H: f64 = 56.0;

// ---------------------------------------------------------------------------
// 列の x 座標計算
// ---------------------------------------------------------------------------

/// 列インデックス col の左上 x 座標を返す。
/// col=0 が起点列、負が先行列、正が後継列。
/// ノード幅は全列固定（NODE_W）。
pub fn col_x(col: i32) -> f64 {
    col as f64 * (NODE_W + H_GAP + HANDLE_W * 2.0)
}

// ---------------------------------------------------------------------------
// GraphNode
// ---------------------------------------------------------------------------

/// `NodeRecord` に Canvas 論理座標を付与した描画・レイアウト用の型。
pub struct GraphNode {
    pub record: NodeRecord,
    /// ノード矩形の左上 x
    pub x: f64,
    /// ノード矩形の左上 y
    pub y: f64,
    /// 所属する列インデックス
    pub col: i32,
}

impl GraphNode {
    pub fn new(record: NodeRecord, col: i32, y: f64) -> Self {
        let x = col_x(col);
        Self { record, x, y, col }
    }

    // --- 座標ヘルパ ---

    pub fn width(&self)  -> f64 { NODE_W }
    pub fn cx(&self)     -> f64 { self.x + NODE_W / 2.0 }
    pub fn cy(&self)     -> f64 { self.y + NODE_H / 2.0 }
    pub fn right(&self)  -> f64 { self.x + NODE_W }
    pub fn bottom(&self) -> f64 { self.y + NODE_H }

    /// ◀ハンドルの論理座標（ノード左端中点の左側）
    pub fn handle_left_center(&self) -> (f64, f64) {
        (self.x - HANDLE_W / 2.0, self.cy())
    }

    /// ▶ハンドルの論理座標（ノード右端中点の右側）
    pub fn handle_right_center(&self) -> (f64, f64) {
        (self.right() + HANDLE_W / 2.0, self.cy())
    }

    // --- ヒットテスト ---

    pub fn hit_body(&self, lx: f64, ly: f64) -> bool {
        lx >= self.x && lx <= self.right() && ly >= self.y && ly <= self.bottom()
    }

    pub fn hit_handle_left(&self, lx: f64, ly: f64) -> bool {
        let (hx, hy) = self.handle_left_center();
        (lx - hx).abs() < HANDLE_W / 2.0 && (ly - hy).abs() < NODE_H / 2.0
    }

    pub fn hit_handle_right(&self, lx: f64, ly: f64) -> bool {
        let (hx, hy) = self.handle_right_center();
        (lx - hx).abs() < HANDLE_W / 2.0 && (ly - hy).abs() < NODE_H / 2.0
    }
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

/// 局所グラフのレイアウト。
/// columns: col → ID リスト（ID 昇順）を管理し、
/// y 座標は columns から都度再計算する。
#[derive(Default)]
pub struct Layout {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<(u32, u32)>,
    /// 列ごとの ID リスト（ID 昇順）
    columns: HashMap<i32, Vec<u32>>,
}

impl Layout {
    /// 起点ノード単体で初期化する（Search 時）。
    pub fn single(root: NodeRecord) -> Self {
        let root_id = root.id;
        let mut columns: HashMap<i32, Vec<u32>> = HashMap::new();
        columns.insert(0, vec![root_id]);
        let nodes = vec![GraphNode::new(root, 0, 0.0)];
        Self { nodes, edges: vec![], columns }
    }

    /// node_id を起点に隣接ノード群を追記する。
    /// is_left=true で先行方向（列-1）、false で後継方向（列+1）。
    /// adj_records: 呼び出し元でキャッシュから取得済みの参照イテレータ。
    /// 重複ノードは除外し、同一列を ID 昇順に再整列する。
    pub fn append<'a>(
        &mut self,
        node_id:     u32,
        is_left:     bool,
        adj_records: impl Iterator<Item = &'a NodeRecord>,
    ) {
        // 起点ノードが Layout 上になければ何もしない
        let anchor_col = match self.nodes.iter().find(|n| n.record.id == node_id) {
            Some(n) => n.col,
            None    => return,
        };

        let target_col = if is_left { anchor_col - 1 } else { anchor_col + 1 };

        // 既存 ID セット（重複チェック用）
        let existing_ids: std::collections::HashSet<u32> =
            self.nodes.iter().map(|n| n.record.id).collect();

        // 新規レコードのみ収集（重複除外、clone は GraphNode 生成時のみ）
        let new_records: Vec<&NodeRecord> = adj_records
            .filter(|r| !existing_ids.contains(&r.id))
            .collect();

        // エッジ追加・columns 更新
        for r in &new_records {
            if is_left {
                self.edges.push((r.id, node_id));
            } else {
                self.edges.push((node_id, r.id));
            }
            self.columns
                .entry(target_col)
                .or_default()
                .push(r.id);
        }

        // target_col の ID リストを昇順ソート
        if let Some(col_ids) = self.columns.get_mut(&target_col) {
            col_ids.sort_unstable();
        }

        // 新規 GraphNode を追加（y は仮値、後で再計算）
        for r in new_records {
            self.nodes.push(GraphNode::new(r.clone(), target_col, 0.0));
        }

        // target_col の全ノードの y 座標を再計算
        self.recalc_col_y(target_col);
    }

    /// 指定列の全ノードを ID 昇順に並べ、y 座標を中央揃えで再計算する。
    fn recalc_col_y(&mut self, col: i32) {
        let col_ids = match self.columns.get(&col) {
            Some(ids) => ids.clone(),
            None      => return,
        };
        let n = col_ids.len();
        let total_h = n as f64 * NODE_H + (n as f64 - 1.0) * V_GAP;
        let y_top   = -(total_h / 2.0);

        for (i, &id) in col_ids.iter().enumerate() {
            let y = y_top + i as f64 * (NODE_H + V_GAP);
            if let Some(gn) = self.nodes.iter_mut().find(|n| n.record.id == id) {
                gn.y = y;
                gn.x = col_x(col); // x も念のため更新
            }
        }
    }
}
