// =============================================================================
// main.rs — DAG Node Viewer (Yew 0.23.0 / CSR / GitHub Pages)
// =============================================================================
// 構成:
//   - ノード数  : 405,348  (u32 ID)
//   - シャーディング: shard_size = 15,625
//     → shard_index = node_id / SHARD_SIZE
//     → ファイルパス: /data/shard_{index}.bin
//   - シリアライズ: rkyv
//
// シャードファイルのデータ構造 (rkyv でシリアライズ済み):
//   Vec<NodeRecord>  ← シャード内のすべてのノードを格納
//
// NodeRecord (rkyv::Archive 実装済みの想定):
//   pub struct NodeRecord {
//       pub id: u32,
//       pub label:   String,
//       pub value_a: u32,   // 仮名: 後で変更
//       pub value_b: u32,   // 仮名: 後で変更
//       pub value_c: u32,   // 仮名: 後で変更
//       pub predecessors: Vec<u32>,
//       pub successors:   Vec<u32>,
//   }
// =============================================================================

use gloo_net::http::Request;
use rkyv::{from_bytes, rancor::Error as RkyvError};
use std::collections::HashMap;
use util::{NodeRecord, SHARD_SIZE, TOTAL_NODES};
use web_sys::HtmlInputElement;
use yew::prelude::*;

// ---------------------------------------------------------------------------
// 定数
// ---------------------------------------------------------------------------
const NODE_HEIGHT: f64 = 40.0;
const CHAR_WIDTH: f64 = 8.5; // フォントサイズに合わせた1文字あたりの横幅
const PADDING: f64 = 24.0; // 左右の余白合計
const HORIZONTAL_SPACING: f64 = 200.0; // ノード間の横間隔
const VERTICAL_SPACING: f64 = 60.0; // ノード間の縦間隔

// GitHub Pages でのデータルート（必要に応じてパスを変更してください）
const DATA_ROOT: &str = "./assets/shards";

// rkyv のデシリアライズを薄くラップするヘルパ
fn deserialize_shard(bytes: &[u8]) -> Result<Vec<NodeRecord>, String> {
    let records =
        from_bytes::<Vec<NodeRecord>, RkyvError>(bytes).map_err(|e| format!("rkyv error: {e}"))?;
    Ok(records)
}

// --- 表示用構造体の定義 ---
#[derive(Clone, PartialEq)]
pub struct RenderNode {
    pub record: NodeRecord,
    pub x: f64,
    pub y: f64,
    pub width: f64,
}

// ---------------------------------------------------------------------------
// メッセージ
// ---------------------------------------------------------------------------
pub enum Msg {
    /// テキストボックスの入力変更
    InputChanged(String),
    /// 検索ボタン押下 / Enter
    Search,
    /// ノードを展開する: ターゲットID, 親の座標(x, y), 子としてのインデックス
    ExpandNode {
        id: u32,
        px: f64,
        py: f64,
        index: usize,
    },
    /// フェッチ成功 → シャードのバイト列を受け取る
    // ShardLoaded(Vec<u8>),
    ShardLoaded {
        bytes: Vec<u8>,
        target_id: u32,
        px: f64,
        py: f64,
        index: usize,
    },
    /// フェッチ失敗
    FetchError(String),
    /// ノードページへジャンプ（検索窓などからの初期エントリ用）
    NavigateTo(u32),
}

// ---------------------------------------------------------------------------
// 状態
// ---------------------------------------------------------------------------
#[derive(Default)]
pub struct App {
    /// テキストボックスの現在値
    input: String,
    /// 現在表示中のノード
    // current_node: Option<NodeRecord>,
    visible_nodes: HashMap<u32, RenderNode>,
    /// ロード中フラグ
    loading: bool,
    /// エラーメッセージ
    error: Option<String>,
    /// シャードキャッシュ: shard_index → Vec<NodeRecord>
    cache: HashMap<u32, Vec<NodeRecord>>,
    // loading: bool,
    // error: Option<String>,
    // キャンバスの移動用（オプション）
    // offset_x: f64,
    // offset_y: f64,
}

impl App {
    fn shard_index(node_id: u32) -> u32 {
        node_id / SHARD_SIZE
    }

    fn shard_url(shard_index: u32) -> String {
        format!("{DATA_ROOT}/shard_{shard_index}.bin")
    }

    /// キャッシュからノードを検索する
    fn find_in_cache(&self, node_id: u32) -> Option<NodeRecord> {
        let idx = Self::shard_index(node_id);
        self.cache
            .get(&idx)?
            .iter()
            .find(|r| r.id == node_id)
            .cloned()
    }
}

// ---------------------------------------------------------------------------
// Component 実装
// ---------------------------------------------------------------------------
impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self::default()
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            // ----------------------------------------------------------------
            Msg::InputChanged(val) => {
                self.input = val;
                false
            }

            // ----------------------------------------------------------------
            Msg::Search => {
                let raw = self.input.trim().to_string();
                let node_id: u32 = match raw.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        self.error = Some(format!("「{raw}」は有効なノード ID ではありません"));
                        // self.current_node = None;
                        return true;
                    }
                };
                if node_id >= TOTAL_NODES {
                    self.error = Some(format!(
                        "ノード ID {node_id} は範囲外です (0 〜 {max})",
                        max = TOTAL_NODES - 1
                    ));
                    // self.current_node = None;
                    return true;
                }

                self.error = None;
                self.visible_nodes.clear();

                // 最初のノードをキャンバスの中央付近に配置
                ctx.link().send_message(Msg::ExpandNode {
                    id: node_id,
                    px: 50.0,  // 初期位置 X
                    py: 300.0, // 初期位置 Y
                    index: 0,  // 最初の要素なのでオフセットなし
                });
                true
            }
            Msg::ExpandNode { id, px, py, index } => {
                // 既に表示されている場合は何もしない（軽量化）
                if self.visible_nodes.contains_key(&id) {
                    return false;
                }

                // キャッシュヒット
                if let Some(record) = self.find_in_cache(id) {
                    // self.current_node = Some(record);
                    // ラベルの長さから動的に幅を計算
                    let width = (record.label.len() as f64 * CHAR_WIDTH) + PADDING;

                    // 親の右方向に配置。index に応じて縦方向にずらす
                    let x = px + HORIZONTAL_SPACING;
                    let y = py + (index as f64 * VERTICAL_SPACING) - VERTICAL_SPACING;

                    self.visible_nodes.insert(
                        id,
                        RenderNode {
                            record,
                            x,
                            y,
                            width,
                        },
                    );
                    self.loading = false;
                    return true;
                }

                // キャッシュミス → フェッチ
                self.loading = true;
                // self.current_node = None;
                let url: String = Self::shard_url(Self::shard_index(id));
                let link = ctx.link().clone();

                wasm_bindgen_futures::spawn_local(async move {
                    match Request::get(&url).send().await {
                        Ok(resp) if resp.ok() => match resp.binary().await {
                            Ok(bytes) => link.send_message(Msg::ShardLoaded {
                                bytes,
                                target_id: id,
                                px,
                                py,
                                index,
                            }),
                            Err(e) => link.send_message(Msg::FetchError(e.to_string())),
                        },
                        Ok(resp) => {
                            link.send_message(Msg::FetchError(format!("HTTP {}", resp.status())));
                        }
                        Err(e) => link.send_message(Msg::FetchError(e.to_string())),
                    }
                });
                true
            }

            // ----------------------------------------------------------------
            Msg::ShardLoaded {
                bytes,
                target_id,
                px,
                py,
                index,
            } => {
                self.loading = false;
                match deserialize_shard(&bytes) {
                    Ok(records) => {
                        // let node_id: u32 = self.input.trim().parse().unwrap_or(0);
                        let shard_idx = Self::shard_index(target_id);
                        // キャッシュへ格納
                        self.cache.insert(shard_idx, records);

                        // 再検索
                        // self.current_node = self.find_in_cache(node_id);
                        // if self.current_node.is_none() {
                        //     self.error =
                        //         Some(format!("ノード {node_id} はシャード内に存在しません"));
                        // }
                        // キャッシュに入れた後、再度展開処理を呼び出す
                        ctx.link().send_message(Msg::ExpandNode {
                            id: target_id,
                            px,
                            py,
                            index,
                        });
                    }
                    Err(e) => {
                        self.error = Some(format!("デシリアライズ失敗: {e}"));
                    }
                }
                true
            }

            // ----------------------------------------------------------------
            Msg::FetchError(e) => {
                self.loading = false;
                self.error = Some(format!("フェッチエラー: {e}"));
                true
            }

            // ----------------------------------------------------------------
            Msg::NavigateTo(node_id) => {
                self.input = node_id.to_string();
                ctx.link().send_message(Msg::Search);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        // ---- 入力ハンドラ ----
        let on_input = link.callback(|e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            Msg::InputChanged(el.value())
        });
        let on_search = link.callback(|_| Msg::Search);
        let on_keydown = link.batch_callback(|e: KeyboardEvent| {
            if e.key() == "Enter" {
                Some(Msg::Search)
            } else {
                None
            }
        });

        // // ---- ノード情報パネル ----
        // let node_panel = if self.loading {
        //     html! {
        //         <div class="panel loading">
        //             <span class="spinner"/>
        //             <p>{"シャードを読み込み中…"}</p>
        //         </div>
        //     }
        // } else if let Some(ref node) = self.current_node {
        //     let preds = node.predecessors.clone();
        //     let succs = node.successors.clone();

        //     let pred_items: Html = if preds.is_empty() {
        //         html! { <li class="empty">{"（なし）"}</li> }
        //     } else {
        //         preds
        //             .iter()
        //             .map(|&pid| {
        //                 let cb = link.callback(move |_| Msg::NavigateTo(pid));
        //                 html! {
        //                     <li key={pid}>
        //                         <button class="node-link" onclick={cb}>
        //                             {format!("#{pid}")}
        //                         </button>
        //                     </li>
        //                 }
        //             })
        //             .collect()
        //     };

        //     let succ_items: Html = if succs.is_empty() {
        //         html! { <li class="empty">{"（なし）"}</li> }
        //     } else {
        //         succs
        //             .iter()
        //             .map(|&sid| {
        //                 let cb = link.callback(move |_| Msg::NavigateTo(sid));
        //                 html! {
        //                     <li key={sid}>
        //                         <button class="node-link" onclick={cb}>
        //                             {format!("#{sid}")}
        //                         </button>
        //                     </li>
        //                 }
        //             })
        //             .collect()
        //     };

        //     html! {
        //         <div class="panel node-panel">
        //             <header class="node-header">
        //                 <span class="node-id-label">{"NODE"}</span>
        //                 <span class="node-id-value">{node.id}</span>
        //                 <span class="node-label">{&node.label}</span>
        //             </header>
        //             <div class="node-values">
        //                 <dl>
        //                     <dt>{"value_a"}</dt><dd>{node.value_a}</dd>
        //                     <dt>{"value_b"}</dt><dd>{node.value_b}</dd>
        //                     <dt>{"value_c"}</dt><dd>{node.value_c}</dd>
        //                 </dl>
        //             </div>
        //             <div class="adjacency-grid">
        //                 <section class="adj-section pred">
        //                     <h3>
        //                         <span class="arrow">{"←"}</span>
        //                         {format!(" 先行ノード（{}件）", preds.len())}
        //                     </h3>
        //                     <ul class="node-list">{pred_items}</ul>
        //                 </section>
        //                 <section class="adj-section succ">
        //                     <h3>
        //                         <span class="arrow">{"→"}</span>
        //                         {format!(" 後継ノード（{}件）", succs.len())}
        //                     </h3>
        //                     <ul class="node-list">{succ_items}</ul>
        //                 </section>
        //             </div>
        //         </div>
        //     }
        // } else {
        //     html! {
        //         <div class="panel empty-panel">
        //             <p>{"ノード ID を入力して検索してください"}</p>
        //             <p class="hint">{format!("有効範囲: 0 〜 {}", TOTAL_NODES - 1)}</p>
        //         </div>
        //     }
        // };
        // エッジのリスト
        let edges = self
            .visible_nodes
            .values()
            .flat_map(|node| {
                node.record.successors.iter().filter_map(|&sid| {
                    self.visible_nodes.get(&sid).map(|target| {
                        html! {
                            <EdgeComponent
                                key={format!("e-{}-{}", node.record.id, sid)}
                                x1={node.x + node.width}
                                y1={node.y + NODE_HEIGHT / 2.0}
                                x2={target.x}
                                y2={target.y + NODE_HEIGHT / 2.0}
                            />
                        }
                    })
                })
            })
            .collect::<Html>();

        // ノードのリスト
        let nodes = self
            .visible_nodes
            .values()
            .map(|node| {
                let n = node.clone();
                let successors = n.record.successors.clone();
                let (nx, ny) = (n.x, n.y);

                let on_click = link.batch_callback(move |_| {
                    successors
                        .iter()
                        .enumerate()
                        .map(|(i, &sid)| Msg::ExpandNode {
                            id: sid,
                            px: nx,
                            py: ny,
                            index: i,
                        })
                        .collect::<Vec<_>>()
                });

                html! {
                    <NodeComponent
                        key={node.record.id}
                        node={node.clone()}
                        on_click={on_click}
                    />
                }
            })
            .collect::<Html>();

        let panel = html! {
            <main class="graph-viewport">
                <svg width="5000" height="5000">
                    <g class="edges-layer">{ edges }</g>
                    <g class="nodes-layer">{ nodes }</g>
                </svg>
            </main>
        };

        // ---- エラー表示 ----
        let error_block = if let Some(ref msg) = self.error {
            html! { <div class="error-banner">{msg}</div> }
        } else {
            html! {}
        };

        // ---- 全体レイアウト ----
        html! {
            <>
                <style>{STYLES}</style>
                <div class="app">
                    <header class="app-header">
                        <h1>{"DAG Explorer"}</h1>
                        <p class="subtitle">
                            {format!("{} nodes · shards of {}", TOTAL_NODES, SHARD_SIZE)}
                        </p>
                    </header>

                    <main class="app-main">
                        <div class="search-row">
                            <input
                                type="number"
                                min="0"
                                max={format!("{}", TOTAL_NODES - 1)}
                                placeholder="ノード ID (例: 42)"
                                value={self.input.clone()}
                                oninput={on_input}
                                onkeydown={on_keydown}
                                class="search-input"
                            />
                            <button class="search-btn" onclick={on_search} disabled={self.loading}>
                                { if self.loading { "Loading…" } else { "Search" } }
                            </button>
                        </div>

                        {error_block}
                        {panel}
                    </main>

                    <footer class="app-footer">
                        {"Powered by Yew 0.23 · rkyv · GitHub Pages"}
                    </footer>
                </div>
            </>
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct NodeProps {
    pub node: RenderNode,
    pub on_click: Callback<MouseEvent>,
}

#[component(NodeComponent)]
pub fn node_component(props: &NodeProps) -> Html {
    let n = &props.node;

    html! {
        <g
            transform={format!("translate({}, {})", n.x, n.y)}
            onclick={props.on_click.clone()}
            class="node-card"
            style="cursor: pointer;"
        >
            <rect
                width={n.width.to_string()}
                height={NODE_HEIGHT.to_string()}
                rx="6"
                fill="var(--surface2)"
                stroke="var(--accent)"
                stroke-width="1"
            />
            <text
                x={(n.width / 2.0).to_string()}
                y={(NODE_HEIGHT / 2.0).to_string()}
                fill="var(--text)"
                font-family="var(--font-mono)"
                font-size="13"
                text-anchor="middle"
                dominant-baseline="central"
            >
                { &n.record.label }
            </text>
        </g>
    }
}

#[derive(Properties, PartialEq)]
pub struct EdgeProps {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

#[function_component(EdgeComponent)]
pub fn edge_component(props: &EdgeProps) -> Html {
    html! {
        <line
            x1={props.x1.to_string()}
            y1={props.y1.to_string()}
            x2={props.x2.to_string()}
            y2={props.y2.to_string()}
            stroke="var(--border)"
            stroke-width="1.5"
            opacity="0.6"
            style="pointer-events: none;"
        />
    }
}

// ---------------------------------------------------------------------------
// CSS (インライン埋め込み)
// ---------------------------------------------------------------------------
const STYLES: &str = r#"
/* ── リセット & ベース ── */
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

:root {
  --bg:        #0d0f14;
  --surface:   #151820;
  --surface2:  #1c2030;
  --border:    #2a3040;
  --accent:    #4fc3f7;
  --accent2:   #ef5350;
  --text:      #e0e6f0;
  --muted:     #6b7a99;
  --radius:    8px;
  --font-mono: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
  --font-sans: 'IBM Plex Sans JP', 'Noto Sans JP', system-ui, sans-serif;
}

body {
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-sans);
  min-height: 100vh;
}

/* ── アプリシェル ── */
.app {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
  max-width: 820px;
  margin: 0 auto;
  padding: 0 1rem;
}

/* ── ヘッダー ── */
.app-header {
  padding: 3rem 0 2rem;
  border-bottom: 1px solid var(--border);
}
.app-header h1 {
  font-family: var(--font-mono);
  font-size: clamp(1.8rem, 5vw, 2.8rem);
  font-weight: 700;
  letter-spacing: -0.03em;
  background: linear-gradient(120deg, var(--accent), #80deea);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}
.subtitle {
  margin-top: .4rem;
  font-family: var(--font-mono);
  font-size: .78rem;
  color: var(--muted);
  letter-spacing: .06em;
}

/* ── メイン ── */
.app-main { flex: 1; padding: 2rem 0; }

/* ── 検索行 ── */
.search-row {
  display: flex;
  gap: .6rem;
  margin-bottom: 1.2rem;
}
.search-input {
  flex: 1;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 1rem;
  padding: .65rem 1rem;
  outline: none;
  transition: border-color .2s;
  /* Chrome の数値スピナーを非表示 */
  -moz-appearance: textfield;
}
.search-input::-webkit-inner-spin-button,
.search-input::-webkit-outer-spin-button { -webkit-appearance: none; }
.search-input:focus { border-color: var(--accent); }
.search-btn {
  background: var(--accent);
  border: none;
  border-radius: var(--radius);
  color: #000;
  cursor: pointer;
  font-family: var(--font-mono);
  font-size: .9rem;
  font-weight: 700;
  letter-spacing: .05em;
  padding: .65rem 1.4rem;
  transition: opacity .15s, transform .1s;
}
.search-btn:hover:not(:disabled) { opacity: .88; transform: translateY(-1px); }
.search-btn:active:not(:disabled) { transform: translateY(0); }
.search-btn:disabled { opacity: .45; cursor: not-allowed; }

/* ── エラー ── */
.error-banner {
  background: rgba(239, 83, 80, .12);
  border: 1px solid var(--accent2);
  border-radius: var(--radius);
  color: #ff8a80;
  font-size: .88rem;
  margin-bottom: 1rem;
  padding: .7rem 1rem;
}

/* ── パネル共通 ── */
.panel {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 1.5rem;
}

/* ── ローディング ── */
.panel.loading {
  display: flex;
  align-items: center;
  gap: 1rem;
  color: var(--muted);
}
.spinner {
  width: 22px; height: 22px;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin .7s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }

/* ── 空パネル ── */
.empty-panel { color: var(--muted); text-align: center; padding: 3rem 1rem; }
.empty-panel .hint { font-size: .82rem; margin-top: .5rem; font-family: var(--font-mono); }

/* ── ノードパネル ── */
.node-header {
  display: flex;
  align-items: baseline;
  gap: .8rem;
  margin-bottom: 1.4rem;
}
.node-id-label {
  font-family: var(--font-mono);
  font-size: .7rem;
  letter-spacing: .15em;
  color: var(--muted);
  background: var(--surface2);
  padding: .2rem .55rem;
  border-radius: 4px;
}
.node-id-value {
  font-family: var(--font-mono);
  font-size: 2rem;
  font-weight: 700;
  color: var(--accent);
}
.node-label {
  font-family: var(--font-sans);
  font-size: 1rem;
  color: var(--text);
  opacity: .75;
  margin-left: .4rem;
}

/* ── ノード値テーブル ── */
.node-values {
  background: var(--surface2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: .8rem 1rem;
  margin-bottom: 1rem;
}
.node-values dl {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: .3rem .8rem;
  align-items: baseline;
}
.node-values dt {
  font-family: var(--font-mono);
  font-size: .78rem;
  color: var(--muted);
  letter-spacing: .04em;
}
.node-values dd {
  font-family: var(--font-mono);
  font-size: .92rem;
  color: var(--text);
}

/* ── 先行/後継グリッド ── */
.adjacency-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;
}
@media (max-width: 540px) {
  .adjacency-grid { grid-template-columns: 1fr; }
}
.adj-section {
  background: var(--surface2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 1rem;
}
.adj-section h3 {
  font-family: var(--font-mono);
  font-size: .8rem;
  font-weight: 600;
  letter-spacing: .05em;
  color: var(--muted);
  margin-bottom: .8rem;
  display: flex;
  align-items: center;
  gap: .3rem;
}
.adj-section.pred .arrow { color: #ef9a9a; }
.adj-section.succ .arrow { color: #80cbc4; }

/* ── ノードリスト ── */
.node-list {
  list-style: none;
  display: flex;
  flex-wrap: wrap;
  gap: .4rem;
  max-height: 260px;
  overflow-y: auto;
  /* スクロールバースタイル */
  scrollbar-width: thin;
  scrollbar-color: var(--border) transparent;
}
.node-list::-webkit-scrollbar { width: 5px; }
.node-list::-webkit-scrollbar-thumb { background: var(--border); border-radius: 3px; }
.node-list li.empty { color: var(--muted); font-size: .84rem; }

/* ── ノードリンクボタン ── */
.node-link {
  background: transparent;
  border: 1px solid var(--border);
  border-radius: 5px;
  color: var(--accent);
  cursor: pointer;
  font-family: var(--font-mono);
  font-size: .82rem;
  padding: .22rem .6rem;
  transition: background .15s, border-color .15s;
}
.node-link:hover {
  background: rgba(79, 195, 247, .1);
  border-color: var(--accent);
}

/* ── フッター ── */
.app-footer {
  border-top: 1px solid var(--border);
  color: var(--muted);
  font-family: var(--font-mono);
  font-size: .72rem;
  letter-spacing: .04em;
  padding: 1.2rem 0;
  text-align: center;
}
"#;
