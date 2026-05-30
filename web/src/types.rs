// =============================================================================
// types.rs — データ型・列挙型・メッセージ定義
// =============================================================================

use web_sys::{MouseEvent, WheelEvent};

// NodeRecord は util クレートで定義する。
// フィールドは非公開。以下のメソッドのみ使用可能:
//   id()           -> u32
//   label()        -> &str
//   values()       -> &[u32]
//   predecessors() -> &[u32]
//   successors()   -> &[u32]
pub use util::NodeRecord;

// ---------------------------------------------------------------------------
// ホバー対象の種別
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum HoverTarget {
    NodeBody(u32),
    HandleLeft(u32),  // « 先行展開 / ‹ 先行収納
    HandleRight(u32), // » 後継展開 / › 後継収納
}

// ---------------------------------------------------------------------------
// Yew メッセージ
// ---------------------------------------------------------------------------

pub enum Msg {
    InputChanged(String),
    Search,
    ExpandLeft(u32),   // «  先行方向へ展開
    ExpandRight(u32),  // »  後継方向へ展開
    CollapseLeft(u32), // ‹  先行方向を収納
    CollapseRight(u32),// ›  後継方向を収納
    MouseDown(MouseEvent),
    MouseMove(MouseEvent),
    MouseUp(MouseEvent),
    MouseLeave,
    Wheel(WheelEvent),
    Resize { w: f64, h: f64 },
    ShardLoaded { triggered_by: u32, bytes: Vec<u8> },
    FetchError   { triggered_by: u32, message: String },
}
