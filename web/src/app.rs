// =============================================================================
// app.rs — App 構造体・Yew Component 実装
// =============================================================================
//
// 変更が必要なケース:
//   - Msg の処理フローを変えるとき
//   - コンテキストメニューの項目を追加するとき
//   - view() の HTML 構造を変えるとき
// =============================================================================

use std::collections::HashMap;

use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::canvas::draw_canvas;
use crate::fetch::{
    SHARD_SIZE, TOTAL_NODES, deserialize_shard, fetch_shard, find_in_cache, shard_index,
};
use crate::layout::{HEADER_H, Layout, POPUP_SCALE_THRESHOLD, ZOOM_MAX, ZOOM_MIN, ZOOM_STEP};
use crate::styles::STYLES;
use crate::types::{ContextAction, HoverTarget, Msg, NodeRecord, SortMode};

// ---------------------------------------------------------------------------
// コンテキストメニューの状態
// ---------------------------------------------------------------------------

struct ContextMenu {
    /// 対象ノードの rep_id
    node_id: u32,
    /// Canvas 上のスクリーン座標
    x: f64,
    y: f64,
    /// 表示する操作項目
    actions: Vec<ContextAction>,
}

// ---------------------------------------------------------------------------
// App 構造体
// ---------------------------------------------------------------------------

pub struct App {
    // 検索
    pub input: String,
    pub error: Option<String>,

    // レイアウト
    pub layout: Layout,

    // シャードキャッシュ: shard_index → Vec<NodeRecord>
    pub cache: HashMap<u32, Vec<NodeRecord>>,
    pub fetching: std::collections::HashSet<u32>,

    // ノード追加待ちキュー: フェッチ完了後に追加するノード ID
    pending_add: std::collections::HashSet<u32>,

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

    // コンテキストメニュー
    context_menu: Option<ContextMenu>,

    // Canvas への NodeRef
    pub canvas_ref: NodeRef,
}

impl Default for App {
    fn default() -> Self {
        Self {
            input: String::new(),
            error: None,
            layout: Layout::default(),
            cache: HashMap::new(),
            fetching: std::collections::HashSet::new(),
            pending_add: std::collections::HashSet::new(),
            pan_x: 0.0,
            pan_y: 0.0,
            scale: 1.0,
            canvas_w: 800.0,
            canvas_h: 600.0,
            drag_start: None,
            hover: None,
            context_menu: None,
            canvas_ref: NodeRef::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// ヘルパ
// ---------------------------------------------------------------------------

impl App {
    fn find_cloned(&self, node_id: u32) -> Option<NodeRecord> {
        find_in_cache(&self.cache, node_id).cloned()
    }

    fn to_logical(&self, sx: f64, sy: f64) -> (f64, f64) {
        let cx = self.canvas_w / 2.0;
        let cy = self.canvas_h / 2.0;
        (
            (sx - cx) / self.scale - self.pan_x,
            (sy - cy) / self.scale - self.pan_y,
        )
    }

    fn hit_test(&self, lx: f64, ly: f64) -> Option<HoverTarget> {
        for gn in &self.layout.nodes {
            // ハンドルを先にチェック
            let has_left = gn
                .kind
                .as_normal()
                .map(|r| !r.predecessors.is_empty())
                .unwrap_or(false);
            let has_right = gn
                .kind
                .as_normal()
                .map(|r| !r.successors.is_empty())
                .unwrap_or(false);

            if has_left && gn.hit_handle_left(lx, ly) {
                return Some(HoverTarget::HandleLeft(gn.rep_id()));
            }
            if has_right && gn.hit_handle_right(lx, ly) {
                return Some(HoverTarget::HandleRight(gn.rep_id()));
            }
            if gn.hit_body(lx, ly) {
                return Some(HoverTarget::NodeBody(gn.rep_id()));
            }
        }
        None
    }

    fn redraw(&self) {
        draw_canvas(
            &self.canvas_ref,
            self.canvas_w,
            self.canvas_h,
            self.pan_x,
            self.pan_y,
            self.scale,
            &self.layout,
            &self.hover,
        );
    }

    /// ノードを追加する（キャッシュ済みなら即座に、未フェッチならフェッチ後に）
    fn add_node_or_fetch(&mut self, ctx: &Context<Self>, node_id: u32) {
        if let Some(record) = self.find_cloned(node_id) {
            self.layout
                .add_node(record, &self.cache, SortMode::default());
        } else if !self.fetching.contains(&node_id) {
            self.fetching.insert(node_id);
            self.pending_add.insert(node_id);
            fetch_shard(ctx.link(), node_id);
        } else {
            // フェッチ中: pending_add に追加してフェッチ完了を待つ
            self.pending_add.insert(node_id);
        }
    }

    /// コンテキストメニューの表示項目を決定する
    fn build_context_actions(&self, node_id: u32) -> Vec<ContextAction> {
        let mut actions = Vec::new();
        let gn = match self.layout.find_node(node_id) {
            Some(g) => g,
            None => return actions,
        };

        // 省略ノードなら展開のみ
        if gn.kind.is_collapsed() {
            actions.push(ContextAction::ExpandCollapsed);
            // collapsed_records が空でない（縮約操作由来）なら可逆展開も追加
            if let crate::types::NodeKind::Collapsed {
                collapsed_records, ..
            } = &gn.kind
            {
                if !collapsed_records.is_empty() {
                    // ExpandCollapsed で統一（UI 上は同じラベルでよい）
                }
            }
            return actions;
        }

        // 通常ノード
        let tier = gn.tier;

        // 仕様3: 同 tier に他ノードが2件以上存在するか
        let others_in_tier = self
            .layout
            .nodes
            .iter()
            .filter(|g| g.tier == tier && g.rep_id() != node_id)
            .count();
        if others_in_tier >= 1 {
            actions.push(ContextAction::CollapseOthersInTier);
        }

        // 仕様4: 複数選択中か
        if self.layout.selected.len() >= 2 && self.layout.is_selected(node_id) {
            actions.push(ContextAction::CollapseSelected);
        }

        // 仕様5: 後継ノードが tier ごとに2件以上存在するか
        let succ_count_by_tier: HashMap<u32, usize> = self
            .layout
            .edges
            .iter()
            .filter(|&&(from, _)| from == node_id)
            .filter_map(|&(_, to)| self.layout.find_node(to).map(|g| g.tier))
            .fold(HashMap::new(), |mut m, t| {
                *m.entry(t).or_insert(0) += 1;
                m
            });
        if succ_count_by_tier.values().any(|&c| c >= 2) {
            actions.push(ContextAction::CollapseSuccsByTier);
        }

        // 仕様6: 先行ノードが tier ごとに2件以上存在するか
        let pred_count_by_tier: HashMap<u32, usize> = self
            .layout
            .edges
            .iter()
            .filter(|&&(_, to)| to == node_id)
            .filter_map(|&(from, _)| self.layout.find_node(from).map(|g| g.tier))
            .fold(HashMap::new(), |mut m, t| {
                *m.entry(t).or_insert(0) += 1;
                m
            });
        if pred_count_by_tier.values().any(|&c| c >= 2) {
            actions.push(ContextAction::CollapsePredsByTier);
        }

        actions
    }

    fn default_sort_mode(&self) -> SortMode {
        SortMode::default()
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
            Msg::AddNode(node_id) => {
                self.error = None;
                self.context_menu = None;

                if node_id >= TOTAL_NODES {
                    self.error = Some(format!(
                        "ノード ID {node_id} は範囲外です (0 〜 {})",
                        TOTAL_NODES - 1
                    ));
                    return true;
                }

                self.add_node_or_fetch(ctx, node_id);
                self.redraw();
                true
            }

            // ----------------------------------------------------------------
            Msg::ClearGraph => {
                self.layout.clear();
                self.pan_x = 0.0;
                self.pan_y = 0.0;
                self.scale = 1.0;
                self.hover = None;
                self.context_menu = None;
                self.pending_add.clear();
                self.redraw();
                true
            }

            // ----------------------------------------------------------------
            Msg::ToggleSelect(rep_id) => {
                self.layout.toggle_select(rep_id);
                self.context_menu = None;
                self.redraw();
                true
            }

            Msg::ClearSelection => {
                self.layout.clear_selection();
                self.redraw();
                true
            }

            // ----------------------------------------------------------------
            Msg::ExpandCollapsed(rep_id) => {
                self.context_menu = None;
                let sort_mode = self.default_sort_mode();
                self.layout.expand_collapsed(rep_id, &self.cache, sort_mode);
                self.redraw();
                true
            }

            // ----------------------------------------------------------------
            Msg::CollapseOthersInTier(node_id) => {
                self.context_menu = None;
                let sort_mode = self.default_sort_mode();
                self.layout.collapse_others_in_tier(node_id, sort_mode);
                self.redraw();
                true
            }

            Msg::CollapseSelected => {
                self.context_menu = None;
                let sort_mode = self.default_sort_mode();
                self.layout.collapse_selected(sort_mode);
                self.redraw();
                true
            }

            Msg::CollapseSuccsByTier(node_id) => {
                self.context_menu = None;
                let sort_mode = self.default_sort_mode();
                self.layout.collapse_succs_by_tier(node_id, sort_mode);
                self.redraw();
                true
            }

            Msg::CollapsePredsByTier(node_id) => {
                self.context_menu = None;
                let sort_mode = self.default_sort_mode();
                self.layout.collapse_preds_by_tier(node_id, sort_mode);
                self.redraw();
                true
            }

            Msg::ExpandCollapsedReversible(rep_id) => {
                self.context_menu = None;
                let sort_mode = self.default_sort_mode();
                self.layout.expand_collapsed_reversible(rep_id, sort_mode);
                self.redraw();
                true
            }

            // ----------------------------------------------------------------
            Msg::CycleSortMode(tier) => {
                self.layout.cycle_sort_mode(tier);
                self.redraw();
                true
            }

            // ----------------------------------------------------------------
            Msg::ShowContextMenu { node_id, x, y } => {
                let actions = self.build_context_actions(node_id);
                self.context_menu = if actions.is_empty() {
                    None
                } else {
                    Some(ContextMenu {
                        node_id,
                        x,
                        y,
                        actions,
                    })
                };
                true
            }

            Msg::HideContextMenu => {
                self.context_menu = None;
                true
            }

            Msg::ContextMenuAction(action) => {
                let node_id = match &self.context_menu {
                    Some(m) => m.node_id,
                    None => return false,
                };
                self.context_menu = None;
                match action {
                    ContextAction::ExpandCollapsed => {
                        ctx.link().send_message(Msg::ExpandCollapsed(node_id))
                    }
                    ContextAction::CollapseOthersInTier => {
                        ctx.link().send_message(Msg::CollapseOthersInTier(node_id))
                    }
                    ContextAction::CollapseSelected => {
                        ctx.link().send_message(Msg::CollapseSelected)
                    }
                    ContextAction::CollapseSuccsByTier => {
                        ctx.link().send_message(Msg::CollapseSuccsByTier(node_id))
                    }
                    ContextAction::CollapsePredsByTier => {
                        ctx.link().send_message(Msg::CollapsePredsByTier(node_id))
                    }
                }
                false
            }

            // ----------------------------------------------------------------
            Msg::MouseDown(e) => {
                // 左クリックでコンテキストメニューを閉じる
                if self.context_menu.is_some() {
                    self.context_menu = None;
                    return true;
                }
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
                let is_click = self.drag_start.map_or(true, |(ssx, ssy, _, _)| {
                    (sx - ssx).abs() < 4.0 && (sy - ssy).abs() < 4.0
                });
                self.drag_start = None;

                if is_click {
                    let (lx, ly) = self.to_logical(sx, sy);
                    match self.hit_test(lx, ly) {
                        Some(HoverTarget::NodeBody(id)) => {
                            ctx.link().send_message(Msg::ToggleSelect(id))
                        }
                        Some(HoverTarget::HandleLeft(id)) => {
                            // «ハンドル: 先行方向へのノード追加（将来的な拡張用）
                            // 現在は何もしない（コンテキストメニューで操作）
                            let _ = id;
                        }
                        Some(HoverTarget::HandleRight(id)) => {
                            let _ = id;
                        }
                        None => {
                            // 空白クリックで選択解除
                            if !self.layout.selected.is_empty() {
                                ctx.link().send_message(Msg::ClearSelection);
                            }
                        }
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
            Msg::ContextMenu(e) => {
                e.prevent_default();
                let (sx, sy) = (e.offset_x() as f64, e.offset_y() as f64);
                let (lx, ly) = self.to_logical(sx, sy);
                if let Some(HoverTarget::NodeBody(id)) = self.hit_test(lx, ly) {
                    ctx.link().send_message(Msg::ShowContextMenu {
                        node_id: id,
                        x: sx,
                        y: sy,
                    });
                } else {
                    self.context_menu = None;
                }
                true
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

                        // pending_add のうちこのシャードで解決できるものを追加
                        let resolved: Vec<u32> = self
                            .pending_add
                            .iter()
                            .copied()
                            .filter(|&id| shard_index(id) == shard_idx)
                            .collect();

                        for id in resolved {
                            self.pending_add.remove(&id);
                            if let Some(record) = self.find_cloned(id) {
                                self.layout
                                    .add_node(record, &self.cache, SortMode::default());
                            }
                        }

                        self.redraw();
                    }
                    Err(e) => {
                        self.error = Some(format!("デシリアライズ失敗: {e}"));
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
                self.pending_add.remove(&triggered_by);
                self.error = Some(format!("フェッチエラー: {message}"));
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
        let on_add = {
            let input = self.input.trim().to_string();
            link.callback(move |_| {
                match input.parse::<u32>() {
                    Ok(id) => Msg::AddNode(id),
                    Err(_) => Msg::AddNode(u32::MAX), // バリデーションエラー用
                }
            })
        };
        let on_keydown = {
            let input = self.input.trim().to_string();
            link.batch_callback(move |e: KeyboardEvent| {
                if e.key() == "Enter" {
                    match input.parse::<u32>() {
                        Ok(id) => Some(Msg::AddNode(id)),
                        Err(_) => Some(Msg::AddNode(u32::MAX)),
                    }
                } else {
                    None
                }
            })
        };
        let on_clear = link.callback(|_| Msg::ClearGraph);
        let on_mousedown = link.callback(Msg::MouseDown);
        let on_mousemove = link.callback(Msg::MouseMove);
        let on_mouseup = link.callback(Msg::MouseUp);
        let on_mouseleave = link.callback(|_| Msg::MouseLeave);
        let on_contextmenu = link.callback(Msg::ContextMenu);
        let on_wheel = link.callback(Msg::Wheel);

        // 凡例
        let legend: Html = util::value_labels()
            .iter()
            .enumerate()
            .map(|(i, &label)| {
                html! {
                    <span class="legend-item" key={i}>{label}</span>
                }
            })
            .collect();

        // 階層別ソートモードボタン（表示中の tier ごとに生成）
        let sort_buttons: Html = {
            let mut tiers: Vec<u32> = self.layout.tiers().collect();
            tiers.sort_unstable();
            tiers
                .iter()
                .map(|&tier| {
                    let mode = self.layout.sort_mode_of(tier);
                    let label = format!("T{} {}", tier, mode.label());
                    let cb = link.callback(move |_| Msg::CycleSortMode(tier));
                    html! {
                        <button class="sort-btn" onclick={cb} key={tier}>{label}</button>
                    }
                })
                .collect()
        };

        // フェッチ中インジケータ
        let loading = if !self.fetching.is_empty() {
            html! { <div class="loading-indicator"><span class="spinner"/></div> }
        } else {
            html! {}
        };

        // エラーバナー
        let error = if let Some(ref msg) = self.error {
            html! { <div class="error-banner">{msg}</div> }
        } else {
            html! {}
        };

        // コンテキストメニュー
        let context_menu = self.view_context_menu(ctx);

        // DOM ポップアップ（縮小時ホバー）
        let popup = self.view_popup();

        html! {
            <>
                <style>{STYLES}</style>
                <div class="app" onclick={link.callback(|_| Msg::HideContextMenu)}>
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
                            <button class="search-btn" onclick={on_add}>{"追加"}</button>
                            <button class="clear-btn"  onclick={on_clear}>{"クリア"}</button>
                        </div>
                        <div class="legend">{legend}</div>
                        <div class="sort-buttons">{sort_buttons}</div>
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
                            oncontextmenu={on_contextmenu}
                            onwheel={on_wheel}
                        />
                        {popup}
                        {context_menu}
                        if self.layout.nodes.is_empty() {
                            <div class="canvas-hint">
                                <p>{"ノード ID を入力して追加してください"}</p>
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
// view のサブ関数
// ---------------------------------------------------------------------------

impl App {
    /// コンテキストメニューの DOM 要素
    fn view_context_menu(&self, ctx: &Context<Self>) -> Html {
        let cm = match &self.context_menu {
            Some(m) => m,
            None => return html! {},
        };

        let style = format!("left:{}px;top:{}px", cm.x, cm.y + HEADER_H);

        let items: Html = cm
            .actions
            .iter()
            .map(|action| {
                let label = match action {
                    ContextAction::ExpandCollapsed => "展開",
                    ContextAction::CollapseOthersInTier => "同階層の他ノードを縮約",
                    ContextAction::CollapseSelected => "選択ノードを縮約",
                    ContextAction::CollapseSuccsByTier => "後継ノードを階層ごとに縮約",
                    ContextAction::CollapsePredsByTier => "先行ノードを階層ごとに縮約",
                };
                let action = action.clone();
                let cb = ctx
                    .link()
                    .callback(move |_| Msg::ContextMenuAction(action.clone()));
                html! {
                    <li class="context-menu-item" onclick={cb}>{label}</li>
                }
            })
            .collect();

        html! {
            <ul class="context-menu" style={style}
                onclick={ctx.link().callback(|e: MouseEvent| { e.stop_propagation(); Msg::HideContextMenu })}
            >
                {items}
            </ul>
        }
    }

    /// 縮小時ホバーの DOM ポップアップ
    fn view_popup(&self) -> Html {
        if self.scale >= POPUP_SCALE_THRESHOLD {
            return html! {};
        }
        let hovered_id = match &self.hover {
            Some(HoverTarget::NodeBody(id)) => *id,
            _ => return html! {},
        };
        let gn = match self.layout.find_node(hovered_id) {
            Some(n) => n,
            None => return html! {},
        };
        let record = match gn.kind.as_normal() {
            Some(r) => r,
            None => return html! {},
        };

        use util::NodeData;
        let cx = self.canvas_w / 2.0;
        let cy = self.canvas_h / 2.0;
        let sx = (gn.cx() + self.pan_x) * self.scale + cx;
        let sy = (gn.y + self.pan_y) * self.scale + cy - 10.0;
        let style = format!(
            "left:{}px;top:{}px;transform:translateX(-50%) translateY(-100%)",
            sx,
            sy + HEADER_H
        );

        let info = record
            .values()
            .map(|v| format!("{:5}", v))
            .collect::<Vec<_>>()
            .join("  ");

        html! {
            <div class="node-popup" style={style}>
                <div class="popup-label">{format!("#{} {}", record.id, record.label)}</div>
                <div class="popup-info">{info}</div>
            </div>
        }
    }
}
