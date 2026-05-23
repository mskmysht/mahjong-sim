// =============================================================================
// app.rs — App 構造体・Yew Component 実装
// =============================================================================
//
// 変更が必要なケース:
//   - メッセージの種類を追加/削除するとき
//   - グラフ操作の挙動（展開方向・リセット条件）を変えるとき
//   - view() の HTML 構造を変えるとき
// =============================================================================

use std::collections::{HashMap, HashSet};

use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::canvas::draw_canvas;
use crate::fetch::{
    SHARD_SIZE, TOTAL_NODES, deserialize_shard, fetch_adjacent, fetch_shard, find_in_cache,
    shard_index,
};
use crate::layout::{HEADER_H, Layout, POPUP_SCALE_THRESHOLD, ZOOM_MAX, ZOOM_MIN, ZOOM_STEP};
use crate::styles::STYLES;
use crate::types::{ExpandDir, HoverTarget, Msg};

use util::NodeRecord;

// ---------------------------------------------------------------------------
// App 構造体
// ---------------------------------------------------------------------------

pub struct App {
    // 検索
    pub input: String,
    pub root_error: Option<String>,

    // 表示中の局所グラフ
    pub root_id: Option<u32>,
    pub expand_dir: ExpandDir,
    pub layout: Layout,

    // シャードキャッシュ: shard_index → Vec<NodeRecord>
    pub cache: HashMap<u32, Vec<NodeRecord>>,
    pub fetching: HashSet<u32>,

    // Canvas ビュー変換
    pub pan_x: f64,
    pub pan_y: f64,
    pub scale: f64,
    pub canvas_w: f64,
    pub canvas_h: f64,

    // ドラッグ: (mouse_x, mouse_y, pan_x_origin, pan_y_origin)
    pub drag_start: Option<(f64, f64, f64, f64)>,

    // ホバー
    pub hover: Option<HoverTarget>,

    // Canvas への NodeRef
    pub canvas_ref: NodeRef,
}

impl Default for App {
    fn default() -> Self {
        Self {
            input: String::new(),
            root_error: None,
            root_id: None,
            expand_dir: ExpandDir::default(),
            layout: Layout::default(),
            cache: HashMap::new(),
            fetching: HashSet::new(),
            pan_x: 0.0,
            pan_y: 0.0,
            scale: 1.0,
            canvas_w: 800.0,
            canvas_h: 600.0,
            drag_start: None,
            hover: None,
            canvas_ref: NodeRef::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// ヘルパ
// ---------------------------------------------------------------------------

impl App {
    /// キャッシュからノードを clone して返す
    fn find_cloned(&self, node_id: u32) -> Option<NodeRecord> {
        find_in_cache(&self.cache, node_id).cloned()
    }

    /// スクリーン座標 → 論理座標（pan/scale の逆変換）
    fn to_logical(&self, sx: f64, sy: f64) -> (f64, f64) {
        let cx = self.canvas_w / 2.0;
        let cy = self.canvas_h / 2.0;
        (
            (sx - cx) / self.scale - self.pan_x,
            (sy - cy) / self.scale - self.pan_y,
        )
    }

    /// ホバー対象を論理座標から判定する。
    /// GraphNode が record を持つため find_in_cache の引き直しが不要。
    fn hit_test(&self, lx: f64, ly: f64) -> Option<HoverTarget> {
        self.root_id?;
        for gn in &self.layout.nodes {
            if !gn.record.predecessors.is_empty() && gn.hit_handle_up(lx, ly) {
                return Some(HoverTarget::HandleUp(gn.record.id));
            }
            if !gn.record.successors.is_empty() && gn.hit_handle_down(lx, ly) {
                return Some(HoverTarget::HandleDown(gn.record.id));
            }
            if gn.hit_body(lx, ly) {
                return Some(HoverTarget::NodeBody(gn.record.id));
            }
        }
        None
    }

    /// キャッシュから Layout を再構築する。
    /// `Layout::from_cache` に委譲することで、app.rs はレイアウト計算の詳細を知らない。
    fn rebuild_layout(&mut self) {
        let root_id = match self.root_id {
            Some(id) => id,
            None => return,
        };
        let root = match self.find_cloned(root_id) {
            Some(r) => r,
            None => return,
        };
        let dir = self.expand_dir;
        self.layout = Layout::from_cache(root, dir, |id| self.find_cloned(id));
    }

    /// Canvas を再描画する。
    /// キャッシュを渡す必要がなくなった（GraphNode が record を所有するため）。
    fn redraw(&self) {
        draw_canvas(
            &self.canvas_ref,
            self.canvas_w,
            self.canvas_h,
            self.pan_x,
            self.pan_y,
            self.scale,
            &self.layout,
            self.root_id,
            &self.hover,
        );
    }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let window = web_sys::window().unwrap();
        let w = window.inner_width().unwrap().as_f64().unwrap_or(800.0);
        let h = window.inner_height().unwrap().as_f64().unwrap_or(600.0);

        // resize イベントリスナー登録
        {
            let link = ctx.link().clone();
            let closure = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
                let win = web_sys::window().unwrap();
                let w = win.inner_width().unwrap().as_f64().unwrap_or(800.0);
                let h = win.inner_height().unwrap().as_f64().unwrap_or(600.0);
                link.send_message(Msg::Resize { w, h });
            });
            window
                .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
        }

        let mut app = Self::default();
        app.canvas_w = w;
        app.canvas_h = h - HEADER_H;
        app
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
                        self.root_error =
                            Some(format!("「{raw}」は有効なノード ID ではありません"));
                        return true;
                    }
                };
                if node_id >= TOTAL_NODES {
                    self.root_error = Some(format!(
                        "ノード ID {node_id} は範囲外です (0 〜 {})",
                        TOTAL_NODES - 1
                    ));
                    return true;
                }
                self.root_error = None;
                self.root_id = Some(node_id);
                self.expand_dir = ExpandDir::Both;
                self.pan_x = 0.0;
                self.pan_y = 0.0;
                self.scale = 1.0;
                self.hover = None;
                self.layout = Layout::default();

                if self.find_cloned(node_id).is_none() && !self.fetching.contains(&node_id) {
                    self.fetching.insert(node_id);
                    fetch_shard(ctx.link(), node_id);
                } else {
                    self.rebuild_layout();
                    self.redraw();
                }
                true
            }

            // ----------------------------------------------------------------
            Msg::ExpandUp(node_id) => {
                self.root_id = Some(node_id);
                self.expand_dir = ExpandDir::Up;
                self.hover = None;

                if let Some(root) = self.find_cloned(node_id) {
                    fetch_adjacent(
                        ctx.link(),
                        &root.predecessors.clone(),
                        &mut self.fetching,
                        &self.cache,
                    );
                    self.rebuild_layout();
                    self.redraw();
                } else if !self.fetching.contains(&node_id) {
                    self.fetching.insert(node_id);
                    fetch_shard(ctx.link(), node_id);
                }
                true
            }

            // ----------------------------------------------------------------
            Msg::ExpandDown(node_id) => {
                self.root_id = Some(node_id);
                self.expand_dir = ExpandDir::Down;
                self.hover = None;

                if let Some(root) = self.find_cloned(node_id) {
                    fetch_adjacent(
                        ctx.link(),
                        &root.successors.clone(),
                        &mut self.fetching,
                        &self.cache,
                    );
                    self.rebuild_layout();
                    self.redraw();
                } else if !self.fetching.contains(&node_id) {
                    self.fetching.insert(node_id);
                    fetch_shard(ctx.link(), node_id);
                }
                true
            }

            // ----------------------------------------------------------------
            Msg::MouseDown(e) => {
                let (sx, sy) = (e.offset_x() as f64, e.offset_y() as f64);
                self.drag_start = Some((sx, sy, self.pan_x, self.pan_y));
                false
            }

            Msg::MouseMove(e) => {
                let (sx, sy) = (e.offset_x() as f64, e.offset_y() as f64);

                if let Some((start_sx, start_sy, orig_px, orig_py)) = self.drag_start {
                    self.pan_x = orig_px + (sx - start_sx) / self.scale;
                    self.pan_y = orig_py + (sy - start_sy) / self.scale;
                    self.redraw();
                    return false;
                }

                let (lx, ly) = self.to_logical(sx, sy);
                let new_hover = self.hit_test(lx, ly);
                if new_hover != self.hover {
                    self.hover = new_hover;
                    self.redraw();
                    return true;
                }
                false
            }

            Msg::MouseUp(e) => {
                let (sx, sy) = (e.offset_x() as f64, e.offset_y() as f64);
                let is_click = self.drag_start.map_or(true, |(start_sx, start_sy, _, _)| {
                    (sx - start_sx).abs() < 4.0 && (sy - start_sy).abs() < 4.0
                });
                self.drag_start = None;

                if is_click {
                    let (lx, ly) = self.to_logical(sx, sy);
                    match self.hit_test(lx, ly) {
                        Some(HoverTarget::HandleUp(id)) => {
                            ctx.link().send_message(Msg::ExpandUp(id))
                        }
                        Some(HoverTarget::HandleDown(id)) => {
                            ctx.link().send_message(Msg::ExpandDown(id))
                        }
                        _ => {}
                    }
                }
                false
            }

            Msg::MouseLeave => {
                self.drag_start = None;
                if self.hover.is_some() {
                    self.hover = None;
                    self.redraw();
                    return true;
                }
                false
            }

            // ----------------------------------------------------------------
            Msg::Wheel(e) => {
                e.prevent_default();
                let factor = if e.delta_y() < 0.0 {
                    1.0 + ZOOM_STEP
                } else {
                    1.0 - ZOOM_STEP
                };
                self.scale = (self.scale * factor).clamp(ZOOM_MIN, ZOOM_MAX);
                self.redraw();
                false
            }

            // ----------------------------------------------------------------
            Msg::Resize { w, h } => {
                self.canvas_w = w;
                self.canvas_h = h - HEADER_H;
                self.redraw();
                true
            }

            // ----------------------------------------------------------------
            Msg::ShardLoaded {
                triggered_by,
                bytes,
            } => {
                self.fetching.remove(&triggered_by);
                let shard_idx = shard_index(triggered_by);

                match deserialize_shard(&bytes) {
                    Ok(records) => {
                        self.cache.insert(shard_idx, records);

                        if let Some(root_id) = self.root_id {
                            // 隣接ノードの未フェッチシャードを追加フェッチ
                            if let Some(root) = self.find_cloned(root_id) {
                                let adj: Vec<u32> = root
                                    .predecessors
                                    .iter()
                                    .chain(root.successors.iter())
                                    .copied()
                                    .collect();
                                fetch_adjacent(ctx.link(), &adj, &mut self.fetching, &self.cache);
                            }
                            self.rebuild_layout();
                            self.redraw();
                        }
                    }
                    Err(e) => {
                        self.root_error = Some(format!("デシリアライズ失敗: {e}"));
                    }
                }
                true
            }

            // ----------------------------------------------------------------
            Msg::FetchError {
                triggered_by,
                message,
            } => {
                self.fetching.remove(&triggered_by);
                self.root_error = Some(format!("フェッチエラー: {message}"));
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        let on_input = link.callback(|e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            Msg::InputChanged(el.value())
        });
        let on_search = link.callback(|_| Msg::Search);
        let on_keydown =
            link.batch_callback(|e: KeyboardEvent| (e.key() == "Enter").then_some(Msg::Search));
        let on_mousedown = link.callback(Msg::MouseDown);
        let on_mousemove = link.callback(Msg::MouseMove);
        let on_mouseup = link.callback(Msg::MouseUp);
        let on_mouseleave = link.callback(|_| Msg::MouseLeave);
        let on_wheel = link.callback(Msg::Wheel);

        let popup = self.view_popup();
        let tooltip = self.view_tooltip();
        let error = if let Some(ref msg) = self.root_error {
            html! { <div class="error-banner">{msg}</div> }
        } else {
            html! {}
        };
        let loading = if !self.fetching.is_empty() {
            html! { <div class="loading-indicator"><span class="spinner"/></div> }
        } else {
            html! {}
        };

        html! {
            <>
                <style>{STYLES}</style>
                <div class="app">
                    <header class="app-header">
                        <span class="app-title">{"DAG Explorer"}</span>
                        <div class="search-row">
                            <input
                                type="number"
                                min="0"
                                max={format!("{}", TOTAL_NODES - 1)}
                                placeholder={format!("ノード ID  (0 〜 {})", TOTAL_NODES - 1)}
                                value={self.input.clone()}
                                oninput={on_input}
                                onkeydown={on_keydown}
                                class="search-input"
                            />
                            <button class="search-btn" onclick={on_search}>{"Search"}</button>
                        </div>
                        {loading}
                    </header>
                    <div class="canvas-wrap">
                        {error}
                        <canvas
                            ref={self.canvas_ref.clone()}
                            width={self.canvas_w.to_string()}
                            height={self.canvas_h.to_string()}
                            class="main-canvas"
                            onmousedown={on_mousedown}
                            onmousemove={on_mousemove}
                            onmouseup={on_mouseup}
                            onmouseleave={on_mouseleave}
                            onwheel={on_wheel}
                        />
                        {popup}
                        {tooltip}
                        if self.root_id.is_none() {
                            <div class="canvas-hint">
                                <p>{"ノード ID を入力して検索してください"}</p>
                                <p class="hint-sub">
                                    {format!("{} nodes · shards of {}", TOTAL_NODES, SHARD_SIZE)}
                                </p>
                            </div>
                        }
                    </div>
                </div>
            </>
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        self.redraw();
    }
}

// ---------------------------------------------------------------------------
// view のサブ関数（ポップアップ・ツールチップ）
// ---------------------------------------------------------------------------

impl App {
    /// 縮小時ホバーで表示する DOM ポップアップ。
    /// GraphNode が record を所有するため、キャッシュを引き直す必要がない。
    fn view_popup(&self) -> Html {
        if self.scale >= POPUP_SCALE_THRESHOLD {
            return html! {};
        }
        let hovered_id = match &self.hover {
            Some(HoverTarget::NodeBody(id)) => *id,
            _ => return html! {},
        };
        let gn = match self.layout.nodes.iter().find(|n| n.record.id == hovered_id) {
            Some(n) => n,
            None => return html! {},
        };

        let cx = self.canvas_w / 2.0;
        let cy = self.canvas_h / 2.0;
        let sx = (gn.cx() + self.pan_x) * self.scale + cx;
        let sy = (gn.y + self.pan_y) * self.scale + cy - 10.0;
        let style = format!(
            "left:{}px;top:{}px;transform:translateX(-50%) translateY(-100%)",
            sx,
            sy + HEADER_H
        );
        html! {
            <div class="node-popup" style={style}>
                <div class="popup-label">{&gn.record.label}</div>
                <div class="popup-info">{gn.record.info_text()}</div>
            </div>
        }
    }

    /// ハンドルホバー時のツールチップ。
    fn view_tooltip(&self) -> Html {
        let (hovered_id, is_up) = match &self.hover {
            Some(HoverTarget::HandleUp(id)) => (*id, true),
            Some(HoverTarget::HandleDown(id)) => (*id, false),
            _ => return html! {},
        };
        let gn = match self.layout.nodes.iter().find(|n| n.record.id == hovered_id) {
            Some(n) => n,
            None => return html! {},
        };

        let count = if is_up {
            gn.record.predecessors.len()
        } else {
            gn.record.successors.len()
        };
        let label = if is_up { "先行" } else { "後継" };

        let (hx, hy) = if is_up {
            gn.handle_up_center()
        } else {
            gn.handle_down_center()
        };
        let cx = self.canvas_w / 2.0;
        let cy = self.canvas_h / 2.0;
        let sx = (hx + self.pan_x) * self.scale + cx;
        let sy = (hy + self.pan_y) * self.scale + cy + HEADER_H;
        let tfm = if is_up {
            "translateX(-50%) translateY(-120%)"
        } else {
            "translateX(-50%) translateY(20%)"
        };

        html! {
            <div class="tooltip" style={format!("left:{}px;top:{}px;transform:{}", sx, sy, tfm)}>
                {format!("{} {}件", label, count)}
            </div>
        }
    }
}
