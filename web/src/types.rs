// =============================================================================
// types.rs — データ型・列挙型・メッセージ定義
// =============================================================================
//
// 変更が必要なケース:
//   - HoverTarget にバリアントを追加するとき
//   - SortMode にバリアントを追加するとき
//   - Msg にメッセージを追加するとき
// =============================================================================

use web_sys::{MouseEvent, WheelEvent};

// ---------------------------------------------------------------------------
// NodeRecord の re-export
// ---------------------------------------------------------------------------

// NodeRecord は util クレートで定義する。フィールドは非公開。
// dag-viewer は NodeData trait 経由でのみアクセスする。
pub use util::NodeRecord;

// ---------------------------------------------------------------------------
// ノードの種別
// ---------------------------------------------------------------------------

/// Canvas 上に表示されるノードの種別。
#[derive(Clone, Debug)]
pub enum NodeKind {
    /// 通常ノード（シャードから読み込んだ実ノード）
    Normal(NodeRecord),

    /// 省略ノード（非表示ノード群のプレースホルダ）
    ///
    /// # 生成タイミング
    /// ノード v を追加したとき、v の非表示な先行・後継ノードを
    /// tier ごとに1件以上まとめて省略ノードとする。
    ///
    /// # 展開
    /// 展開操作は不可逆。hidden_ids の各ノードを通常ノードとして追加する。
    ///
    /// # 縮約（仕様3〜6）
    /// 縮約操作は可逆。collapsed_nodes に元の GraphNode を保持し、
    /// 展開操作で復元できる。
    Collapsed {
        /// この省略ノードが代理する非表示ノードの ID 群
        hidden_ids: Vec<u32>,
        /// 縮約操作で格納された元の GraphNode 群（展開で復元する）
        /// 追加ノードに付随する省略ノードでは空
        collapsed_records: Vec<NodeRecord>,
    },
}

impl NodeKind {
    /// 通常ノードの場合は Some(&NodeRecord)、省略ノードの場合は None を返す
    pub fn as_normal(&self) -> Option<&NodeRecord> {
        match self {
            NodeKind::Normal(r) => Some(r),
            NodeKind::Collapsed { .. } => None,
        }
    }

    /// 省略ノードかどうか
    pub fn is_collapsed(&self) -> bool {
        matches!(self, NodeKind::Collapsed { .. })
    }

    /// ノードを一意に識別する ID を返す。
    /// 通常ノード: NodeRecord::id()
    /// 省略ノード: hidden_ids の最小値（一意性の保証用）
    pub fn representative_id(&self) -> u32 {
        match self {
            NodeKind::Normal(r) => r.id,
            NodeKind::Collapsed { hidden_ids, .. } => {
                hidden_ids.iter().copied().min().unwrap_or(u32::MAX)
            }
        }
    }

    /// 省略ノードの hidden_ids を返す（通常ノードは空スライス）
    pub fn hidden_ids(&self) -> &[u32] {
        match self {
            NodeKind::Normal(_) => &[],
            NodeKind::Collapsed { hidden_ids, .. } => hidden_ids,
        }
    }
}

// ---------------------------------------------------------------------------
// ソートモード
// ---------------------------------------------------------------------------

/// 階層内のノードのソート順。
/// 各 tier ごとに独立して設定できる。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SortMode {
    /// 階層内全体を util::compare_nodes() でソート（デフォルト）
    #[default]
    Global,
    /// 先行ノードの sort_key を優先してグループ化
    PredFirst,
    /// 後継ノードの sort_key を優先してグループ化
    SuccFirst,
}

impl SortMode {
    pub fn label(self) -> &'static str {
        match self {
            SortMode::Global => "全体",
            SortMode::PredFirst => "先行優先",
            SortMode::SuccFirst => "後継優先",
        }
    }

    /// 次のモードに切り替える（循環）
    pub fn next(self) -> Self {
        match self {
            SortMode::Global => SortMode::PredFirst,
            SortMode::PredFirst => SortMode::SuccFirst,
            SortMode::SuccFirst => SortMode::Global,
        }
    }
}

// ---------------------------------------------------------------------------
// ホバー対象の種別
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum HoverTarget {
    /// 通常ノードまたは省略ノードの本体
    NodeBody(u32), // representative_id
    /// 先行方向ハンドル
    HandleLeft(u32), // representative_id
    /// 後継方向ハンドル
    HandleRight(u32), // representative_id
}

// ---------------------------------------------------------------------------
// コンテキストメニューの項目
// ---------------------------------------------------------------------------

/// 右クリックコンテキストメニューに表示する操作。
/// 対象ノードと表示中のグラフの状態によって表示項目が変わる。
#[derive(Clone, Debug, PartialEq)]
pub enum ContextAction {
    /// 省略ノードを展開（不可逆）
    ExpandCollapsed,
    /// 選択ノードvと同tierの他ノード全部を縮約（仕様3）
    CollapseOthersInTier,
    /// 複数選択ノードを縮約（仕様4）
    CollapseSelected,
    /// vの各tierの後継2件以上を縮約（仕様5）
    CollapseSuccsByTier,
    /// vの各tierの先行2件以上を縮約（仕様6）
    CollapsePredsByTier,
}

// ---------------------------------------------------------------------------
// Yew メッセージ
// ---------------------------------------------------------------------------

pub enum Msg {
    // --- 検索・グラフ操作 ---
    InputChanged(String),
    /// ノードを表示領域に追加する
    AddNode(u32),
    /// 表示領域をクリア（リセット）
    ClearGraph,

    // --- ノード選択 ---
    /// ノードを選択/解除（複数選択可能）
    ToggleSelect(u32),
    /// 選択をすべて解除
    ClearSelection,

    // --- 省略ノード操作 ---
    /// 省略ノードを展開（不可逆）
    ExpandCollapsed(u32), // representative_id

    // --- 縮約操作（可逆） ---
    /// 選択ノードと同tierの他ノード全部を縮約（仕様3）
    CollapseOthersInTier(u32),
    /// 複数選択ノードを縮約（仕様4）
    CollapseSelected,
    /// vの各tierの後継2件以上を縮約（仕様5）
    CollapseSuccsByTier(u32),
    /// vの各tierの先行2件以上を縮約（仕様6）
    CollapsePredsByTier(u32),
    /// 縮約された省略ノードを元の通常ノード群に戻す（仕様3〜6の逆操作）
    ExpandCollapsedReversible(u32),

    // --- ソートモード ---
    /// 指定 tier のソートモードを次に切り替える
    CycleSortMode(u32),

    // --- コンテキストメニュー ---
    ShowContextMenu {
        node_id: u32,
        x: f64,
        y: f64,
    },
    HideContextMenu,
    ContextMenuAction(ContextAction),

    // --- Canvas インタラクション ---
    MouseDown(MouseEvent),
    MouseMove(MouseEvent),
    MouseUp(MouseEvent),
    MouseLeave,
    ContextMenu(MouseEvent),
    Wheel(WheelEvent),

    // --- ウィンドウ ---
    Resize {
        w: f64,
        h: f64,
    },

    // --- フェッチ ---
    ShardLoaded {
        triggered_by: u32,
        bytes: Vec<u8>,
    },
    FetchError {
        triggered_by: u32,
        message: String,
    },
}
