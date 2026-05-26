// =============================================================================
// types.rs — データ型・列挙型・メッセージ定義
// =============================================================================
//
// 変更が必要なケース:
//   - HoverTarget / ExpandDir にバリアントを追加するとき
//   - Msg にメッセージを追加するとき
//
// NodeRecord は shared_types クレートに属する。
// 描画・レイアウト関連のメソッドは GraphNode (layout.rs) に集約する。
// =============================================================================

use web_sys::{MouseEvent, WheelEvent};
use util::NodeRecord;


// ---------------------------------------------------------------------------
// ホバー対象の種別
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum HoverTarget {
    NodeBody(u32),
    HandleUp(u32),   // ▲
    HandleDown(u32), // ▼
}

// ---------------------------------------------------------------------------
// 展開方向
// ---------------------------------------------------------------------------

// #[derive(Clone, Copy, PartialEq, Default)]
// pub enum ExpandDir {
//     Up,
//     Down,
//     #[default]
//     Both,
// }

// ---------------------------------------------------------------------------
// Yew メッセージ
// ---------------------------------------------------------------------------

pub enum Msg {
    // 検索
    InputChanged(String),
    Search,

    // グラフ操作
    /// ▲ハンドルクリック: 起点を node_id に更新し先行群を表示
    ExpandUp(u32),
    /// ▼ハンドルクリック: 起点を node_id に更新し後継群を表示
    ExpandDown(u32),

    // Canvas インタラクション
    MouseDown(MouseEvent),
    MouseMove(MouseEvent),
    MouseUp(MouseEvent),
    MouseLeave,
    Wheel(WheelEvent),

    // ウィンドウリサイズ
    Resize { w: f64, h: f64 },

    // フェッチ
    ShardLoaded { triggered_by: u32, bytes: Vec<u8> },
    FetchError   { triggered_by: u32, message: String },
}
