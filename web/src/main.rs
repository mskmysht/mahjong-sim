// =============================================================================
// main.rs — エントリーポイント
// =============================================================================
//
// モジュール構成:
//   types   — データ型・列挙型・メッセージ定義（NodeRecord は util クレートから re-export）
//   layout  — レイアウト定数・座標計算
//   fetch   — HTTP フェッチ・rkyv デシリアライズ
//   canvas  — Canvas 描画ロジック
//   app     — App 構造体・Yew Component 実装
//   styles  — CSS 文字列定数
// =============================================================================

mod app;
mod canvas;
mod fetch;
mod layout;
mod styles;
mod types;

fn main() {
    yew::Renderer::<app::App>::new().render();
}
