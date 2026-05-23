// =============================================================================
// canvas.rs — Canvas 描画ロジック
// =============================================================================
//
// 変更が必要なケース:
//   - ノードの見た目（色・角丸・フォント）を変えるとき
//   - エッジの形状（ベジェ曲線の制御点）を変えるとき
//   - ハンドルの記号・サイズを変えるとき
// =============================================================================

use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::layout::{
    EDGE_CTRL_DY, HANDLE_H, INFO_FONT_SIZE, LABEL_FONT_SIZE, LABEL_LINE_H,
    GraphNode, Layout, NODE_H, NODE_PADDING_X, NODE_PADDING_Y,
};
use crate::types::HoverTarget;

// ---------------------------------------------------------------------------
// 描画エントリーポイント
// ---------------------------------------------------------------------------

/// Canvas 全体を再描画する。
/// キャッシュへの参照が不要になった（GraphNode が NodeRecord を所有するため）。
pub fn draw_canvas(
    canvas_ref: &yew::NodeRef,
    canvas_w:   f64,
    canvas_h:   f64,
    pan_x:      f64,
    pan_y:      f64,
    scale:      f64,
    layout:     &Layout,
    root_id:    Option<u32>,
    hover:      &Option<HoverTarget>,
) {
    let canvas = match canvas_ref.cast::<HtmlCanvasElement>() {
        Some(c) => c,
        None    => return,
    };
    let ctx = match canvas
        .get_context("2d").ok().flatten()
        .and_then(|o| o.dyn_into::<CanvasRenderingContext2d>().ok())
    {
        Some(c) => c,
        None    => return,
    };

    // クリア
    ctx.clear_rect(0.0, 0.0, canvas_w, canvas_h);
    ctx.set_fill_style_str("#0d0f14");
    ctx.fill_rect(0.0, 0.0, canvas_w, canvas_h);

    if layout.nodes.is_empty() { return; }

    // ビュー変換: canvas 中央を原点として pan・scale を適用
    ctx.save();
    ctx.translate(canvas_w / 2.0, canvas_h / 2.0).unwrap();
    ctx.scale(scale, scale).unwrap();
    ctx.translate(pan_x, pan_y).unwrap();

    // エッジ描画
    for &(from_id, to_id) in &layout.edges {
        let from = layout.nodes.iter().find(|n| n.record.id == from_id);
        let to   = layout.nodes.iter().find(|n| n.record.id == to_id);
        if let (Some(f), Some(t)) = (from, to) {
            draw_edge(&ctx, f, t, scale);
        }
    }

    // ノード描画
    let root_id = root_id.unwrap_or(u32::MAX);
    for gn in &layout.nodes {
        let is_root    = gn.record.id == root_id;
        let is_hovered = matches!(hover, Some(HoverTarget::NodeBody(id)) if *id == gn.record.id);
        draw_node(&ctx, gn, is_root, is_hovered, hover, scale);
    }

    ctx.restore();
}

// ---------------------------------------------------------------------------
// エッジ
// ---------------------------------------------------------------------------

fn draw_edge(ctx: &CanvasRenderingContext2d, from: &GraphNode, to: &GraphNode, scale: f64) {
    let x1 = from.cx();
    let y1 = from.bottom() + HANDLE_H;
    let x2 = to.cx();
    let y2 = to.y - HANDLE_H;

    ctx.begin_path();
    ctx.move_to(x1, y1);
    ctx.bezier_curve_to(
        x1, y1 + EDGE_CTRL_DY,
        x2, y2 - EDGE_CTRL_DY,
        x2, y2,
    );
    ctx.set_stroke_style_str("#2a3040");
    ctx.set_line_width(1.5 / scale);
    ctx.stroke();

    // 矢じり
    let arrow_size = 7.0 / scale;
    ctx.begin_path();
    ctx.move_to(x2, y2);
    ctx.line_to(x2 - arrow_size / 2.0, y2 - arrow_size);
    ctx.line_to(x2 + arrow_size / 2.0, y2 - arrow_size);
    ctx.close_path();
    ctx.set_fill_style_str("#2a3040");
    ctx.fill();
}

// ---------------------------------------------------------------------------
// ノード
// ---------------------------------------------------------------------------

fn draw_node(
    ctx:        &CanvasRenderingContext2d,
    gn:         &GraphNode,
    is_root:    bool,
    is_hovered: bool,
    hover:      &Option<HoverTarget>,
    scale:      f64,
) {
    let x = gn.x;
    let y = gn.y;
    let w = gn.width();
    let h = NODE_H;
    let r = 6.0;

    let bg     = if is_root { "#1e2640" } else { "#151820" };
    let border = if is_root || is_hovered { "#4fc3f7" } else { "#2a3040" };

    draw_rounded_rect(ctx, x, y, w, h, r);
    ctx.set_fill_style_str(bg);
    ctx.fill();
    ctx.set_stroke_style_str(border);
    ctx.set_line_width(if is_root { 2.0 } else { 1.0 } / scale);
    ctx.stroke();

    // ラベル行
    let text_x = x + NODE_PADDING_X;
    ctx.set_fill_style_str("#e0e6f0");
    ctx.set_font(&format!("{}px 'IBM Plex Sans JP', sans-serif", LABEL_FONT_SIZE));
    ctx.set_text_align("left");
    ctx.set_text_baseline("alphabetic");
    let _ = ctx.fill_text(&gn.record.label, text_x, y + NODE_PADDING_Y + LABEL_FONT_SIZE);

    // 補足情報行
    ctx.set_fill_style_str("#6b7a99");
    ctx.set_font(&format!("{}px 'JetBrains Mono', monospace", INFO_FONT_SIZE));
    let _ = ctx.fill_text(&gn.record.info_text(), text_x, y + NODE_PADDING_Y + LABEL_LINE_H + INFO_FONT_SIZE);

    // ▲ハンドル
    if !gn.record.predecessors.is_empty() {
        let (hx, hy) = gn.handle_up_center();
        let hovered  = matches!(hover, Some(HoverTarget::HandleUp(id)) if *id == gn.record.id);
        draw_handle(ctx, hx, hy, "▲", hovered, scale);
    }

    // ▼ハンドル
    if !gn.record.successors.is_empty() {
        let (hx, hy) = gn.handle_down_center();
        let hovered  = matches!(hover, Some(HoverTarget::HandleDown(id)) if *id == gn.record.id);
        draw_handle(ctx, hx, hy, "▼", hovered, scale);
    }
}

// ---------------------------------------------------------------------------
// ハンドル
// ---------------------------------------------------------------------------

fn draw_handle(
    ctx:     &CanvasRenderingContext2d,
    cx:      f64,
    cy:      f64,
    symbol:  &str,
    hovered: bool,
    scale:   f64,
) {
    ctx.set_fill_style_str(if hovered { "#4fc3f7" } else { "#3a4a60" });
    ctx.set_font(&format!("{}px sans-serif", 12.0 / scale.max(0.5)));
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    let _ = ctx.fill_text(symbol, cx, cy);
    ctx.set_text_align("left");
    ctx.set_text_baseline("alphabetic");
}

// ---------------------------------------------------------------------------
// 角丸矩形ヘルパ
// ---------------------------------------------------------------------------

fn draw_rounded_rect(ctx: &CanvasRenderingContext2d, x: f64, y: f64, w: f64, h: f64, r: f64) {
    use std::f64::consts::{FRAC_PI_2, PI};
    ctx.begin_path();
    ctx.move_to(x + r, y);
    ctx.line_to(x + w - r, y);
    ctx.arc(x + w - r, y + r,     r, -FRAC_PI_2,      0.0,            ).unwrap();
    ctx.line_to(x + w, y + h - r);
    ctx.arc(x + w - r, y + h - r, r,  0.0,             FRAC_PI_2,     ).unwrap();
    ctx.line_to(x + r, y + h);
    ctx.arc(x + r,     y + h - r, r,  FRAC_PI_2,       PI,            ).unwrap();
    ctx.line_to(x, y + r);
    ctx.arc(x + r,     y + r,     r,  PI,              FRAC_PI_2 * 3.0, ).unwrap();
    ctx.close_path();
}
