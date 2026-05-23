// =============================================================================
// styles.rs — CSS 文字列定数
// =============================================================================
//
// 変更が必要なケース:
//   - 色・フォント・余白などのビジュアルを調整するとき
//   - HEADER_H (layout.rs) を変更したとき（--header-h と合わせること）
// =============================================================================

pub const STYLES: &str = r#"
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

:root {
  --bg:        #0d0f14;
  --surface:   #151820;
  --border:    #2a3040;
  --accent:    #4fc3f7;
  --accent2:   #ef5350;
  --text:      #e0e6f0;
  --muted:     #6b7a99;
  --header-h:  56px;   /* layout.rs の HEADER_H と合わせること */
  --radius:    6px;
  --font-mono: 'JetBrains Mono', 'Fira Code', monospace;
  --font-sans: 'IBM Plex Sans JP', 'Noto Sans JP', system-ui, sans-serif;
}

body {
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-sans);
  overflow: hidden;
}

/* ── アプリシェル ── */
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
}

/* ── ヘッダー ── */
.app-header {
  height: var(--header-h);
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0 1.2rem;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  position: relative;
  z-index: 10;
}
.app-title {
  font-family: var(--font-mono);
  font-size: 1rem;
  font-weight: 700;
  letter-spacing: -0.02em;
  background: linear-gradient(120deg, var(--accent), #80deea);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  white-space: nowrap;
}

/* ── 検索行 ── */
.search-row {
  display: flex;
  gap: .5rem;
  flex: 1;
  max-width: 480px;
}
.search-input {
  flex: 1;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: .88rem;
  padding: .4rem .8rem;
  outline: none;
  transition: border-color .2s;
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
  font-size: .82rem;
  font-weight: 700;
  padding: .4rem 1rem;
  transition: opacity .15s;
  white-space: nowrap;
}
.search-btn:hover { opacity: .85; }

/* ── ローディングインジケータ ── */
.loading-indicator {
  margin-left: auto;
  display: flex;
  align-items: center;
}
.spinner {
  width: 18px; height: 18px;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin .7s linear infinite;
  display: inline-block;
}
@keyframes spin { to { transform: rotate(360deg); } }

/* ── Canvas 領域 ── */
.canvas-wrap {
  flex: 1;
  position: relative;
  overflow: hidden;
}
.main-canvas {
  display: block;
  cursor: grab;
}
.main-canvas:active { cursor: grabbing; }

/* ── ヒント（未検索時） ── */
.canvas-hint {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: .5rem;
  color: var(--muted);
  pointer-events: none;
}
.hint-sub {
  font-family: var(--font-mono);
  font-size: .76rem;
  opacity: .6;
}

/* ── エラーバナー ── */
.error-banner {
  position: absolute;
  top: .8rem;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(239,83,80,.15);
  border: 1px solid var(--accent2);
  border-radius: var(--radius);
  color: #ff8a80;
  font-size: .84rem;
  padding: .5rem 1rem;
  z-index: 20;
  white-space: nowrap;
}

/* ── ノードポップアップ（縮小時ホバー） ── */
.node-popup {
  position: absolute;
  background: #1c2030;
  border: 1px solid var(--accent);
  border-radius: var(--radius);
  padding: .5rem .8rem;
  pointer-events: none;
  z-index: 30;
  white-space: nowrap;
}
.popup-label {
  font-family: var(--font-sans);
  font-size: 14px;
  color: var(--text);
}
.popup-info {
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--muted);
  margin-top: 2px;
}

/* ── ツールチップ（ハンドルホバー） ── */
.tooltip {
  position: absolute;
  background: #1c2030;
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--muted);
  font-family: var(--font-mono);
  font-size: .76rem;
  padding: .25rem .6rem;
  pointer-events: none;
  z-index: 30;
  white-space: nowrap;
}
"#;
