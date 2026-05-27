// =============================================================================
// canvas.rs — Canvas 描画ロジック
// =============================================================================
//
// 変更が必要なケース:
//   - ノードの見た目（色・角丸・フォント）を変えるとき
//   - エッジの形状を変えるとき
//   - ハンドルの記号・サイズを変えるとき
// =============================================================================

use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::layout::{
    EDGE_CTRL_DX, GraphNode, HANDLE_W, ID_FONT_SIZE, INFO_FONT_SIZE, LABEL_FONT_SIZE, LABEL_LINE_H,
    Layout, NODE_H, NODE_PADDING_X, NODE_PADDING_Y,
};
use crate::types::HoverTarget;

// ---------------------------------------------------------------------------
// 描画エントリーポイント
// ---------------------------------------------------------------------------

pub fn draw_canvas(
    canvas_ref: &yew::NodeRef,
    canvas_w: f64,
    canvas_h: f64,
    pan_x: f64,
    pan_y: f64,
    scale: f64,
    layout: &Layout,
    root_id: Option<u32>,
    hover: &Option<HoverTarget>,
) {
    let canvas = match canvas_ref.cast::<HtmlCanvasElement>() {
        Some(c) => c,
        None => return,
    };
    let ctx = match canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|o| o.dyn_into::<CanvasRenderingContext2d>().ok())
    {
        Some(c) => c,
        None => return,
    };

    ctx.clear_rect(0.0, 0.0, canvas_w, canvas_h);
    ctx.set_fill_style_str("#0d0f14");
    ctx.fill_rect(0.0, 0.0, canvas_w, canvas_h);

    if layout.nodes.is_empty() {
        return;
    }

    ctx.save();
    ctx.translate(canvas_w / 2.0, canvas_h / 2.0).unwrap();
    ctx.scale(scale, scale).unwrap();
    ctx.translate(pan_x, pan_y).unwrap();

    // エッジ（ノードより先に描画してノードに隠れるようにする）
    for &(from_id, to_id) in &layout.edges {
        let from = layout.nodes.iter().find(|n| n.record.id == from_id);
        let to = layout.nodes.iter().find(|n| n.record.id == to_id);
        if let (Some(f), Some(t)) = (from, to) {
            draw_edge(&ctx, f, t, scale);
        }
    }

    // ノード
    let root_id = root_id.unwrap_or(u32::MAX);
    for gn in &layout.nodes {
        let is_root = gn.record.id == root_id;
        let is_hovered = matches!(hover, Some(HoverTarget::NodeBody(id)) if *id == gn.record.id);
        draw_node(&ctx, gn, is_root, is_hovered, hover, scale);
    }

    ctx.restore();
}

// ---------------------------------------------------------------------------
// エッジ（水平ベジェ曲線）
// ---------------------------------------------------------------------------

fn draw_edge(ctx: &CanvasRenderingContext2d, from: &GraphNode, to: &GraphNode, scale: f64) {
    // from の右端中点 → to の左端中点
    let x1 = from.right() + HANDLE_W;
    let y1 = from.cy();
    let x2 = to.x - HANDLE_W;
    let y2 = to.cy();

    ctx.begin_path();
    ctx.move_to(x1, y1);
    ctx.bezier_curve_to(x1 + EDGE_CTRL_DX, y1, x2 - EDGE_CTRL_DX, y2, x2, y2);
    ctx.set_stroke_style_str("#2a3040");
    ctx.set_line_width(1.5 / scale);
    ctx.stroke();

    // 矢じり（to の左端向き）
    let arrow_size = 7.0 / scale;
    ctx.begin_path();
    ctx.move_to(x2, y2);
    ctx.line_to(x2 - arrow_size, y2 - arrow_size / 2.0);
    ctx.line_to(x2 - arrow_size, y2 + arrow_size / 2.0);
    ctx.close_path();
    ctx.set_fill_style_str("#2a3040");
    ctx.fill();
}

// ---------------------------------------------------------------------------
// ノード
// ---------------------------------------------------------------------------

fn draw_node(
    ctx: &CanvasRenderingContext2d,
    gn: &GraphNode,
    is_root: bool,
    is_hovered: bool,
    hover: &Option<HoverTarget>,
    scale: f64,
) {
    let x = gn.x;
    let y = gn.y;
    let w = gn.width();
    let h = NODE_H;

    let bg = if is_root { "#1e2640" } else { "#151820" };
    let border = if is_root || is_hovered {
        "#4fc3f7"
    } else {
        "#2a3040"
    };

    draw_rounded_rect(ctx, x, y, w, h, 6.0);
    ctx.set_fill_style_str(bg);
    ctx.fill();
    ctx.set_stroke_style_str(border);
    ctx.set_line_width(if is_root { 2.0 } else { 1.0 } / scale);
    ctx.stroke();

    let text_x = x + NODE_PADDING_X;

    // ID（小さくラベルの上に表示）
    ctx.set_fill_style_str("#4b5570");
    ctx.set_font(&format!("{}px 'JetBrains Mono', monospace", ID_FONT_SIZE));
    ctx.set_text_align("left");
    ctx.set_text_baseline("alphabetic");
    let _ = ctx.fill_text(
        &format!("#{}", gn.record.id),
        text_x,
        y + ID_FONT_SIZE + 2.0,
    );

    // ラベル行
    ctx.set_fill_style_str("#e0e6f0");
    ctx.set_font(&format!(
        "{}px 'IBM Plex Sans JP', sans-serif",
        LABEL_FONT_SIZE
    ));
    let _ = ctx.fill_text(
        &gn.record.label,
        text_x,
        y + NODE_PADDING_Y + LABEL_FONT_SIZE,
    );

    // 補足情報行
    ctx.set_fill_style_str("#6b7a99");
    ctx.set_font(&format!("{}px 'JetBrains Mono', monospace", INFO_FONT_SIZE));
    let _ = ctx.fill_text(
        &gn.record.info_text(),
        text_x,
        y + NODE_PADDING_Y + LABEL_LINE_H + INFO_FONT_SIZE,
    );

    // ◀ハンドル（先行が存在する場合）
    if !gn.record.predecessors.is_empty() {
        let (hx, hy) = gn.handle_left_center();
        let hovered = matches!(hover, Some(HoverTarget::HandleLeft(id)) if *id == gn.record.id);
        draw_handle(ctx, hx, hy, "◀", hovered, scale);
    }

    // ▶ハンドル（後継が存在する場合）
    if !gn.record.successors.is_empty() {
        let (hx, hy) = gn.handle_right_center();
        let hovered = matches!(hover, Some(HoverTarget::HandleRight(id)) if *id == gn.record.id);
        draw_handle(ctx, hx, hy, "▶", hovered, scale);
    }
}

// ---------------------------------------------------------------------------
// ハンドル
// ---------------------------------------------------------------------------

fn draw_handle(
    ctx: &CanvasRenderingContext2d,
    cx: f64,
    cy: f64,
    symbol: &str,
    hovered: bool,
    scale: f64,
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
    ctx.arc(x + w - r, y + r, r, -FRAC_PI_2, 0.0).unwrap();
    ctx.line_to(x + w, y + h - r);
    ctx.arc(x + w - r, y + h - r, r, 0.0, FRAC_PI_2).unwrap();
    ctx.line_to(x + r, y + h);
    ctx.arc(x + r, y + h - r, r, FRAC_PI_2, PI).unwrap();
    ctx.line_to(x, y + r);
    ctx.arc(x + r, y + r, r, PI, FRAC_PI_2 * 3.0).unwrap();
    ctx.close_path();
}
