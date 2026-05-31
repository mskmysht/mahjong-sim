// =============================================================================
// styles.rs — CSS 文字列定数
// =============================================================================
//
// 変更が必要なケース:
//   - 色・フォント・余白などのビジュアルを調整するとき
//   - HEADER_H (layout.rs) を変更したとき（--header-h と合わせること）
//   - コンテキストメニュー・ソートボタンのスタイルを調整するとき
// =============================================================================

pub const STYLES: &str = r#"
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
  gap: .8rem;
  padding: 0 1.2rem;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  position: relative;
  z-index: 10;
  overflow: hidden; /* tier が増えても崩れないよう */
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
  flex-shrink: 0;
}

/* ── 検索行 ── */
.search-row {
  display: flex;
  gap: .4rem;
  flex-shrink: 0;
}

.search-input {
  width: 180px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: .82rem;
  padding: .35rem .7rem;
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
  font-size: .78rem;
  font-weight: 700;
  padding: .35rem .8rem;
  transition: opacity .15s;
  white-space: nowrap;
  flex-shrink: 0;
}
.search-btn:hover { opacity: .85; }

.clear-btn {
  background: transparent;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--muted);
  cursor: pointer;
  font-family: var(--font-mono);
  font-size: .78rem;
  padding: .35rem .8rem;
  transition: border-color .15s, color .15s;
  white-space: nowrap;
  flex-shrink: 0;
}
.clear-btn:hover {
  border-color: var(--accent2);
  color: #ff8a80;
}

/* ── 凡例パネル ── */
.legend {
  display: flex;
  align-items: center;
  gap: .5rem;
  flex-shrink: 0;
  border-left: 1px solid var(--border);
  padding-left: .8rem;
}
.legend-item {
  font-family: var(--font-mono);
  font-size: .72rem;
  color: var(--muted);
  white-space: nowrap;
}

/* ── 階層別ソートボタン ── */
.sort-buttons {
  display: flex;
  align-items: center;
  gap: .3rem;
  overflow-x: auto;
  flex-shrink: 1;
  min-width: 0;
  scrollbar-width: none;
}
.sort-buttons::-webkit-scrollbar { display: none; }

.sort-btn {
  background: var(--surface2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--muted);
  cursor: pointer;
  font-family: var(--font-mono);
  font-size: .70rem;
  padding: .25rem .55rem;
  white-space: nowrap;
  flex-shrink: 0;
  transition: border-color .15s, color .15s;
}
.sort-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
}

/* ── ローディングインジケータ ── */
.loading-indicator {
  margin-left: auto;
  display: flex;
  align-items: center;
  flex-shrink: 0;
}
.spinner {
  width: 16px; height: 16px;
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

/* ── ヒント（未追加時） ── */
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
  font-size: .74rem;
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
  font-size: .82rem;
  padding: .45rem .9rem;
  z-index: 20;
  white-space: nowrap;
  pointer-events: none;
}

/* ── コンテキストメニュー ── */
.context-menu {
  position: absolute;
  background: var(--surface2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  list-style: none;
  min-width: 160px;
  z-index: 50;
  box-shadow: 0 4px 16px rgba(0,0,0,.4);
  overflow: hidden;
}
.context-menu-item {
  padding: .5rem .9rem;
  font-family: var(--font-sans);
  font-size: .84rem;
  color: var(--text);
  cursor: pointer;
  transition: background .1s;
  white-space: nowrap;
}
.context-menu-item:hover {
  background: rgba(79,195,247,.1);
  color: var(--accent);
}
.context-menu-item + .context-menu-item {
  border-top: 1px solid var(--border);
}

/* ── ノードポップアップ（縮小時ホバー） ── */
.node-popup {
  position: absolute;
  background: var(--surface2);
  border: 1px solid var(--accent);
  border-radius: var(--radius);
  padding: .45rem .75rem;
  pointer-events: none;
  z-index: 30;
  white-space: nowrap;
  box-shadow: 0 2px 8px rgba(0,0,0,.3);
}
.popup-label {
  font-family: var(--font-sans);
  font-size: 13px;
  color: var(--text);
}
.popup-info {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--muted);
  margin-top: 3px;
}
"#;
