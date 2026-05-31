// =============================================================================
// layout.rs — レイアウト定数・GraphNode・Layout
// =============================================================================

use std::collections::{HashMap, HashSet};

use util::{NodeData, NodeRecord, compare_nodes};

use crate::fetch::find_in_cache;
use crate::types::{NodeKind, SortMode};

// ---------------------------------------------------------------------------
// 描画定数
// ---------------------------------------------------------------------------

pub const NODE_PADDING_X: f64 = 12.0;
pub const NODE_PADDING_Y: f64 = 8.0;
pub const LABEL_FONT_SIZE: f64 = 14.0;
pub const INFO_FONT_SIZE: f64 = 12.0;
pub const LABEL_LINE_H: f64 = 20.0;
pub const INFO_LINE_H: f64 = 18.0;
pub const NODE_H: f64 = NODE_PADDING_Y * 2.0 + LABEL_LINE_H + INFO_LINE_H;
pub const FULL_ANGLE_CHAR_W: f64 = LABEL_FONT_SIZE;
pub const LABEL_MAX_CHARS: f64 = 14.0;
pub const LABEL_MAX_W: f64 = FULL_ANGLE_CHAR_W * LABEL_MAX_CHARS;
pub const INFO_ITEM_W: f64 = 50.0;
pub const NODE_W: f64 = LABEL_MAX_W + NODE_PADDING_X * 2.0;
pub const COLLAPSED_W: f64 = 80.0;
pub const COLLAPSED_H: f64 = NODE_H;

// ---------------------------------------------------------------------------
// レイアウト定数
// ---------------------------------------------------------------------------

pub const H_GAP: f64 = 60.0;
pub const V_GAP: f64 = 12.0;
pub const HANDLE_W: f64 = 20.0;
pub const EDGE_CTRL_DX: f64 = 40.0;
pub const ID_FONT_SIZE: f64 = 10.0;

// ---------------------------------------------------------------------------
// ズーム定数
// ---------------------------------------------------------------------------

pub const ZOOM_MIN: f64 = 0.2;
pub const ZOOM_MAX: f64 = 3.0;
pub const ZOOM_STEP: f64 = 0.1;
pub const POPUP_SCALE_THRESHOLD: f64 = 0.8;
pub const HEADER_H: f64 = 56.0;

// ---------------------------------------------------------------------------
// tier の x 座標
// ---------------------------------------------------------------------------

pub fn tier_x(tier: u32) -> f64 {
    tier as f64 * (NODE_W + H_GAP + HANDLE_W * 2.0)
}

// ---------------------------------------------------------------------------
// GraphNode
// ---------------------------------------------------------------------------

pub struct GraphNode {
    pub kind: NodeKind,
    pub x: f64,
    pub y: f64,
    pub tier: u32,
}

impl GraphNode {
    pub fn new_normal(record: NodeRecord, tier: u32, y: f64) -> Self {
        Self {
            kind: NodeKind::Normal(record),
            x: tier_x(tier),
            y,
            tier,
        }
    }

    pub fn new_collapsed(
        hidden_ids: Vec<u32>,
        collapsed_records: Vec<NodeRecord>,
        tier: u32,
        y: f64,
    ) -> Self {
        Self {
            kind: NodeKind::Collapsed {
                hidden_ids,
                collapsed_records,
            },
            x: tier_x(tier) + (NODE_W - COLLAPSED_W) / 2.0,
            y,
            tier,
        }
    }

    pub fn width(&self) -> f64 {
        if self.kind.is_collapsed() {
            COLLAPSED_W
        } else {
            NODE_W
        }
    }
    pub fn height(&self) -> f64 {
        NODE_H
    }
    pub fn cx(&self) -> f64 {
        self.x + self.width() / 2.0
    }
    pub fn cy(&self) -> f64 {
        self.y + self.height() / 2.0
    }
    pub fn right(&self) -> f64 {
        self.x + self.width()
    }
    pub fn bottom(&self) -> f64 {
        self.y + self.height()
    }

    pub fn handle_left_center(&self) -> (f64, f64) {
        (tier_x(self.tier) - HANDLE_W / 2.0, self.cy())
    }
    pub fn handle_right_center(&self) -> (f64, f64) {
        (tier_x(self.tier) + NODE_W + HANDLE_W / 2.0, self.cy())
    }

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

    pub fn rep_id(&self) -> u32 {
        self.kind.representative_id()
    }
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct Layout {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<(u32, u32)>,
    pub selected: HashSet<u32>,
    tiers: HashMap<u32, TierState>,
}

struct TierState {
    order: Vec<u32>,
    sort_mode: SortMode,
}

impl TierState {
    fn new(sort_mode: SortMode) -> Self {
        Self {
            order: Vec::new(),
            sort_mode,
        }
    }
}

// ---------------------------------------------------------------------------
// キャッシュ型エイリアス（借用の明示）
// ---------------------------------------------------------------------------

type Cache = HashMap<u32, Vec<NodeRecord>>;

// ---------------------------------------------------------------------------
// Layout のメソッド
// ---------------------------------------------------------------------------

impl Layout {
    // -----------------------------------------------------------------------
    // 公開: ノード追加
    // -----------------------------------------------------------------------

    /// ノード v を表示領域に追加する。
    /// `cache` は `App::cache` への参照を直接受け取る（クロージャなし）。
    pub fn add_node(&mut self, record: NodeRecord, cache: &Cache, sort_mode: SortMode) {
        let id = record.id;
        let tier = record.tier();

        if self.find_node(id).is_some() {
            return;
        }

        self.insert_node(GraphNode::new_normal(record.clone(), tier, 0.0), sort_mode);
        self.connect_to_existing(&record);

        // 非表示先行・後継を省略ノードとして追加（借用コンフリクト回避のため
        // group_hidden_by_tier の結果を先に収集してから self を mut 借用する）
        let hidden_preds = self.group_hidden_by_tier(&record.predecessors, cache);
        let hidden_succs = self.group_hidden_by_tier(&record.successors, cache);

        self.attach_collapsed(id, hidden_preds, true, sort_mode);
        self.attach_collapsed(id, hidden_succs, false, sort_mode);

        let affected_tiers: Vec<u32> = self.tiers.keys().copied().collect();
        for t in affected_tiers {
            self.recalc_tier_y(t);
        }
    }

    /// 省略ノードを展開する（不可逆）。
    pub fn expand_collapsed(&mut self, rep_id: u32, cache: &Cache, sort_mode: SortMode) {
        let hidden_ids: Vec<u32> = match self.find_node(rep_id) {
            Some(gn) => match &gn.kind {
                NodeKind::Collapsed { hidden_ids, .. } => hidden_ids.clone(),
                _ => return,
            },
            None => return,
        };

        if hidden_ids.is_empty() {
            return;
        }

        self.remove_node(rep_id);

        // hidden_ids を事前に収集してから add_node を呼ぶ
        let records: Vec<NodeRecord> = hidden_ids
            .iter()
            .filter_map(|&id| find_in_cache(cache, id).cloned())
            .collect();
        for record in records {
            self.add_node(record, cache, sort_mode);
        }
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.selected.clear();
        self.tiers.clear();
    }

    // -----------------------------------------------------------------------
    // 公開: 選択操作
    // -----------------------------------------------------------------------

    pub fn toggle_select(&mut self, rep_id: u32) {
        if !self.selected.remove(&rep_id) {
            self.selected.insert(rep_id);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    // -----------------------------------------------------------------------
    // 公開: 縮約操作（可逆）
    // -----------------------------------------------------------------------

    pub fn collapse_others_in_tier(&mut self, v_rep_id: u32, sort_mode: SortMode) {
        let tier = match self.find_node(v_rep_id) {
            Some(gn) => gn.tier,
            None => return,
        };
        let others: Vec<u32> = self
            .nodes
            .iter()
            .filter(|gn| gn.tier == tier && gn.rep_id() != v_rep_id)
            .map(|gn| gn.rep_id())
            .collect();
        if others.len() < 2 {
            return;
        }
        self.collapse_nodes_into_one(others, tier, sort_mode);
    }

    pub fn collapse_selected(&mut self, sort_mode: SortMode) {
        let selected: Vec<u32> = self.selected.iter().copied().collect();
        if selected.len() < 2 {
            return;
        }
        let tier = match self.find_node(selected[0]) {
            Some(gn) => gn.tier,
            None => return,
        };
        if selected
            .iter()
            .any(|&id| self.find_node(id).map(|gn| gn.tier) != Some(tier))
        {
            return;
        }
        self.collapse_nodes_into_one(selected, tier, sort_mode);
        self.selected.clear();
    }

    pub fn collapse_succs_by_tier(&mut self, v_rep_id: u32, sort_mode: SortMode) {
        let succ_ids: Vec<u32> = self
            .edges
            .iter()
            .filter(|&&(from, _)| from == v_rep_id)
            .map(|&(_, to)| to)
            .collect();
        self.collapse_adj_by_tier(succ_ids, sort_mode);
    }

    pub fn collapse_preds_by_tier(&mut self, v_rep_id: u32, sort_mode: SortMode) {
        let pred_ids: Vec<u32> = self
            .edges
            .iter()
            .filter(|&&(_, to)| to == v_rep_id)
            .map(|&(from, _)| from)
            .collect();
        self.collapse_adj_by_tier(pred_ids, sort_mode);
    }

    pub fn expand_collapsed_reversible(&mut self, rep_id: u32, sort_mode: SortMode) {
        let (collapsed_records, tier) = match self.find_node(rep_id) {
            Some(gn) => match &gn.kind {
                NodeKind::Collapsed {
                    collapsed_records, ..
                } => (collapsed_records.clone(), gn.tier),
                _ => return,
            },
            None => return,
        };

        self.remove_node(rep_id);

        for record in collapsed_records {
            let tier_r = record.tier();
            self.insert_node(
                GraphNode::new_normal(record.clone(), tier_r, 0.0),
                sort_mode,
            );
            self.connect_to_existing(&record);
        }
        self.recalc_tier_y(tier);
    }

    // -----------------------------------------------------------------------
    // 公開: ソートモード切り替え
    // -----------------------------------------------------------------------

    pub fn cycle_sort_mode(&mut self, tier: u32) {
        let new_mode = match self.tiers.get(&tier) {
            Some(ts) => ts.sort_mode.next(),
            None => return,
        };
        if let Some(ts) = self.tiers.get_mut(&tier) {
            ts.sort_mode = new_mode;
        }
        self.sort_tier(tier, new_mode);
        self.recalc_tier_y(tier);
    }

    pub fn sort_mode_of(&self, tier: u32) -> SortMode {
        self.tiers
            .get(&tier)
            .map(|ts| ts.sort_mode)
            .unwrap_or_default()
    }

    // -----------------------------------------------------------------------
    // 公開: クエリ
    // -----------------------------------------------------------------------

    pub fn find_node(&self, rep_id: u32) -> Option<&GraphNode> {
        self.nodes.iter().find(|gn| gn.rep_id() == rep_id)
    }

    pub fn is_selected(&self, rep_id: u32) -> bool {
        self.selected.contains(&rep_id)
    }

    pub fn highlighted_edges(&self) -> HashSet<(u32, u32)> {
        self.edges
            .iter()
            .filter(|&&(from, to)| self.selected.contains(&from) || self.selected.contains(&to))
            .copied()
            .collect()
    }

    pub fn tiers(&self) -> impl Iterator<Item = u32> + '_ {
        self.tiers.keys().copied()
    }

    // -----------------------------------------------------------------------
    // 内部: ノード挿入・削除
    // -----------------------------------------------------------------------

    fn insert_node(&mut self, gn: GraphNode, sort_mode: SortMode) {
        let tier = gn.tier;
        let rep_id = gn.rep_id();
        self.nodes.push(gn);
        self.tiers
            .entry(tier)
            .or_insert_with(|| TierState::new(sort_mode))
            .order
            .push(rep_id);
        // sort_tier は &self.nodes を参照するため order を clone してから実行
        self.sort_tier(tier, sort_mode);
    }

    fn remove_node(&mut self, rep_id: u32) {
        let tier = match self.find_node(rep_id) {
            Some(gn) => gn.tier,
            None => return,
        };
        self.nodes.retain(|gn| gn.rep_id() != rep_id);
        self.edges.retain(|&(f, t)| f != rep_id && t != rep_id);
        self.selected.remove(&rep_id);
        if let Some(ts) = self.tiers.get_mut(&tier) {
            ts.order.retain(|&id| id != rep_id);
        }
        self.recalc_tier_y(tier);
    }

    // -----------------------------------------------------------------------
    // 内部: 接続関係の構築
    // -----------------------------------------------------------------------

    fn connect_to_existing(&mut self, record: &NodeRecord) {
        let id = record.id;
        let mut new_edges: Vec<(u32, u32)> = Vec::new();

        // 先行ノードのうち表示中のものとエッジを張る
        for &pred_id in &record.predecessors {
            if self.find_node(pred_id).is_some() {
                new_edges.push((pred_id, id));
            }
        }
        // 後継ノードのうち表示中のものとエッジを張る
        for &succ_id in &record.successors {
            if self.find_node(succ_id).is_some() {
                new_edges.push((id, succ_id));
            }
        }
        // 既存省略ノードとの接続（仕様3.2）
        let collapsed_info: Vec<(u32, Vec<u32>)> = self
            .nodes
            .iter()
            .filter(|gn| gn.kind.is_collapsed())
            .map(|gn| (gn.rep_id(), gn.kind.hidden_ids().to_vec()))
            .collect();
        for (rep, hidden) in collapsed_info {
            if record.predecessors.iter().any(|p| hidden.contains(p)) {
                new_edges.push((rep, id));
            }
            if record.successors.iter().any(|s| hidden.contains(s)) {
                new_edges.push((id, rep));
            }
        }

        for e in new_edges {
            if !self.edges.contains(&e) {
                self.edges.push(e);
            }
        }
    }

    /// 省略ノードを tier ごとに生成して接続する共通処理。
    /// `is_pred=true` で先行方向、`false` で後継方向。
    fn attach_collapsed(
        &mut self,
        node_id: u32,
        hidden_by_tier: HashMap<u32, Vec<u32>>,
        is_pred: bool,
        sort_mode: SortMode,
    ) {
        for (tier, hidden_ids) in hidden_by_tier {
            let (absorbed, new_ids) = self.split_by_overlap(&hidden_ids);

            for rep_id in absorbed {
                let e = if is_pred {
                    (rep_id, node_id)
                } else {
                    (node_id, rep_id)
                };
                if !self.edges.contains(&e) {
                    self.edges.push(e);
                }
            }

            if !new_ids.is_empty() {
                let gn = GraphNode::new_collapsed(new_ids, vec![], tier, 0.0);
                let rep = gn.rep_id();
                self.insert_node(gn, sort_mode);
                let e = if is_pred {
                    (rep, node_id)
                } else {
                    (node_id, rep)
                };
                if !self.edges.contains(&e) {
                    self.edges.push(e);
                }
            }
        }
    }

    /// 隣接 ID 群のうち非表示のものを tier ごとにグループ化する。
    /// `cache` を直接受け取ることで `&self` との借用コンフリクトを回避する。
    fn group_hidden_by_tier(&self, adj_ids: &[u32], cache: &Cache) -> HashMap<u32, Vec<u32>> {
        let visible: HashSet<u32> = self
            .nodes
            .iter()
            .filter_map(|gn| gn.kind.as_normal().map(|r| r.id))
            .collect();

        let mut by_tier: HashMap<u32, Vec<u32>> = HashMap::new();
        for &adj_id in adj_ids {
            if visible.contains(&adj_id) {
                continue;
            }
            if self
                .nodes
                .iter()
                .any(|gn| gn.kind.hidden_ids().contains(&adj_id))
            {
                continue;
            }
            if let Some(r) = find_in_cache(cache, adj_id) {
                by_tier.entry(r.tier()).or_default().push(adj_id);
            }
        }
        by_tier
    }

    fn split_by_overlap(&self, candidate_ids: &[u32]) -> (Vec<u32>, Vec<u32>) {
        let mut absorbed_reps: Vec<u32> = Vec::new();
        let mut remaining: Vec<u32> = candidate_ids.to_vec();

        for gn in self.nodes.iter().filter(|gn| gn.kind.is_collapsed()) {
            let hidden = gn.kind.hidden_ids();
            if remaining.iter().any(|id| hidden.contains(id)) {
                absorbed_reps.push(gn.rep_id());
                remaining.retain(|id| !hidden.contains(id));
            }
        }
        (absorbed_reps, remaining)
    }

    // -----------------------------------------------------------------------
    // 内部: 縮約ヘルパ
    // -----------------------------------------------------------------------

    fn collapse_nodes_into_one(&mut self, rep_ids: Vec<u32>, tier: u32, sort_mode: SortMode) {
        // 収集フェーズ（&self）
        let mut collapsed_records: Vec<NodeRecord> = Vec::new();
        let mut incoming: Vec<u32> = Vec::new();
        let mut outgoing: Vec<u32> = Vec::new();

        for &rep_id in &rep_ids {
            if let Some(r) = self.find_node(rep_id).and_then(|gn| gn.kind.as_normal()) {
                collapsed_records.push(r.clone());
            }
            for &(from, to) in &self.edges {
                if to == rep_id && !rep_ids.contains(&from) && !incoming.contains(&from) {
                    incoming.push(from);
                }
                if from == rep_id && !rep_ids.contains(&to) && !outgoing.contains(&to) {
                    outgoing.push(to);
                }
            }
        }

        // 削除フェーズ（&mut self）
        for &rep_id in &rep_ids {
            self.nodes.retain(|gn| gn.rep_id() != rep_id);
            self.edges.retain(|&(f, t)| f != rep_id && t != rep_id);
            self.selected.remove(&rep_id);
            if let Some(ts) = self.tiers.get_mut(&tier) {
                ts.order.retain(|&id| id != rep_id);
            }
        }

        // 省略ノード生成
        let gn = GraphNode::new_collapsed(vec![], collapsed_records, tier, 0.0);
        let rep = gn.rep_id();
        self.insert_node(gn, sort_mode);

        for from in incoming {
            let e = (from, rep);
            if !self.edges.contains(&e) {
                self.edges.push(e);
            }
        }
        for to in outgoing {
            let e = (rep, to);
            if !self.edges.contains(&e) {
                self.edges.push(e);
            }
        }

        self.recalc_tier_y(tier);
    }

    fn collapse_adj_by_tier(&mut self, adj_rep_ids: Vec<u32>, sort_mode: SortMode) {
        // 収集フェーズ（&self）
        let by_tier: HashMap<u32, Vec<u32>> = adj_rep_ids
            .iter()
            .filter_map(|&id| self.find_node(id).map(|gn| (gn.tier, id)))
            .fold(HashMap::new(), |mut m, (t, id)| {
                m.entry(t).or_default().push(id);
                m
            });
        // 変更フェーズ（&mut self）
        for (tier, ids) in by_tier {
            if ids.len() >= 2 {
                self.collapse_nodes_into_one(ids, tier, sort_mode);
            }
        }
    }

    // -----------------------------------------------------------------------
    // 内部: ソート・y 座標再計算
    // -----------------------------------------------------------------------

    fn sort_tier(&mut self, tier: u32, mode: SortMode) {
        // order と mode を clone して &self.tiers の借用を終わらせる
        let mut order = match self.tiers.get(&tier) {
            Some(ts) => ts.order.clone(),
            None => return,
        };

        // この時点で self.tiers の借用は終了 → self.nodes を自由に参照できる
        match mode {
            SortMode::Global => {
                order.sort_by(|&a, &b| {
                    let gn_a = self.nodes.iter().find(|gn| gn.rep_id() == a);
                    let gn_b = self.nodes.iter().find(|gn| gn.rep_id() == b);
                    match (gn_a, gn_b) {
                        (Some(ga), Some(gb)) => match (ga.kind.as_normal(), gb.kind.as_normal()) {
                            (Some(ra), Some(rb)) => compare_nodes(ra, rb),
                            (Some(_), None) => std::cmp::Ordering::Less,
                            (None, Some(_)) => std::cmp::Ordering::Greater,
                            (None, None) => a.cmp(&b),
                        },
                        _ => a.cmp(&b),
                    }
                });
            }
            SortMode::PredFirst => {
                order.sort_by(|&a, &b| {
                    let ka = self.min_pred_key(a, tier);
                    let kb = self.min_pred_key(b, tier);
                    ka.cmp(&kb).then_with(|| a.cmp(&b))
                });
            }
            SortMode::SuccFirst => {
                order.sort_by(|&a, &b| {
                    let ka = self.min_succ_key(a, tier);
                    let kb = self.min_succ_key(b, tier);
                    ka.cmp(&kb).then_with(|| a.cmp(&b))
                });
            }
        }

        // ソート結果を書き戻す
        if let Some(ts) = self.tiers.get_mut(&tier) {
            ts.order = order;
        }
    }

    fn min_pred_key(&self, rep_id: u32, tier: u32) -> u32 {
        self.edges
            .iter()
            .filter(|&&(_, to)| to == rep_id)
            .filter_map(|&(from, _)| {
                self.find_node(from)
                    .filter(|gn| gn.tier < tier)
                    .map(|_| from)
            })
            .min()
            .unwrap_or(u32::MAX)
    }

    fn min_succ_key(&self, rep_id: u32, tier: u32) -> u32 {
        self.edges
            .iter()
            .filter(|&&(from, _)| from == rep_id)
            .filter_map(|&(_, to)| self.find_node(to).filter(|gn| gn.tier > tier).map(|_| to))
            .min()
            .unwrap_or(u32::MAX)
    }

    fn recalc_tier_y(&mut self, tier: u32) {
        let order = match self.tiers.get(&tier) {
            Some(ts) => ts.order.clone(),
            None => return,
        };
        let mut y = 0.0;
        for rep_id in &order {
            if let Some(gn) = self.nodes.iter_mut().find(|gn| gn.rep_id() == *rep_id) {
                gn.y = y;
                gn.x = if gn.kind.is_collapsed() {
                    tier_x(tier) + (NODE_W - COLLAPSED_W) / 2.0
                } else {
                    tier_x(tier)
                };
                y += gn.height() + V_GAP;
            }
        }
    }
}
