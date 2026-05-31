// =============================================================================
// fetch.rs — HTTP フェッチ・rkyv デシリアライズ
// =============================================================================
//
// 変更が必要なケース:
//   - NodeRecord のシリアライズ形式を変更するとき
//   - シャードの URL 規則を変更するとき
//   - フェッチのエラーハンドリングを変更するとき
// =============================================================================

use std::collections::{HashMap, HashSet};

use gloo_net::http::Request;
use yew::html::Scope;

use crate::types::{Msg, NodeRecord};

// ---------------------------------------------------------------------------
// 定数
// ---------------------------------------------------------------------------

pub const SHARD_SIZE:  u32  = 15_625;
pub const TOTAL_NODES: u32  = 405_348;
pub const DATA_ROOT:   &str = "/data";

// ---------------------------------------------------------------------------
// シャードユーティリティ
// ---------------------------------------------------------------------------

pub fn shard_index(node_id: u32) -> u32 {
    node_id / SHARD_SIZE
}

pub fn shard_url(idx: u32) -> String {
    format!("{DATA_ROOT}/shard_{idx}.bin")
}

// ---------------------------------------------------------------------------
// デシリアライズ
// ---------------------------------------------------------------------------

/// シャードバイト列を Vec<NodeRecord> にデシリアライズする。
///
/// # 本番実装
/// ```rust
/// rkyv::from_bytes::<Vec<NodeRecord>, rkyv::rancor::Error>(bytes)
///     .map_err(|e| format!("rkyv error: {e}"))
/// ```
pub fn deserialize_shard(bytes: &[u8]) -> Result<Vec<NodeRecord>, String> {
    // --- プレースホルダ ---
    let _ = bytes;
    Ok(vec![])
}

// ---------------------------------------------------------------------------
// フェッチ
// ---------------------------------------------------------------------------

/// node_id が属するシャードを非同期フェッチし、
/// 完了時に `ShardLoaded` / `FetchError` メッセージを送る。
pub fn fetch_shard<COMP>(link: &Scope<COMP>, node_id: u32)
where
    COMP: yew::Component<Message = Msg>,
{
    let url  = shard_url(shard_index(node_id));
    let link = link.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match Request::get(&url).send().await {
            Ok(resp) if resp.ok() => match resp.binary().await {
                Ok(bytes) => link.send_message(Msg::ShardLoaded { triggered_by: node_id, bytes }),
                Err(e)    => link.send_message(Msg::FetchError   { triggered_by: node_id, message: e.to_string() }),
            },
            Ok(resp) => link.send_message(Msg::FetchError {
                triggered_by: node_id,
                message: format!("HTTP {}", resp.status()),
            }),
            Err(e) => link.send_message(Msg::FetchError { triggered_by: node_id, message: e.to_string() }),
        }
    });
}

/// 隣接ノード群のうち未キャッシュ・未フェッチのシャードをまとめてフェッチする。
/// 同一シャード内の複数ノードは 1 回のフェッチに集約される。
pub fn fetch_adjacent<COMP>(
    link:     &Scope<COMP>,
    adj_ids:  &[u32],
    fetching: &mut HashSet<u32>,
    cache:    &HashMap<u32, Vec<NodeRecord>>,
)
where
    COMP: yew::Component<Message = Msg>,
{
    let mut shards_to_fetch: HashSet<u32> = HashSet::new();
    for &id in adj_ids {
        let shard = shard_index(id);
        if !cache.contains_key(&shard) && !fetching.contains(&id) {
            shards_to_fetch.insert(shard);
            fetching.insert(id);
        }
    }
    for shard in shards_to_fetch {
        // シャード内の代表 ID としてシャード先頭を使用
        fetch_shard(link, shard * SHARD_SIZE);
    }
}

/// キャッシュから node_id のレコードを検索する。
pub fn find_in_cache<'a>(
    cache:   &'a HashMap<u32, Vec<NodeRecord>>,
    node_id: u32,
) -> Option<&'a NodeRecord> {
    cache
        .get(&shard_index(node_id))?
        .iter()
        .find(|r| r.id == node_id)
}

/// adj_ids のうちキャッシュ済みのレコードへの参照をイテレータで返す。
/// 中間 Vec を生成せず、clone も行わない。
/// clone が必要な場合は呼び出し元（Layout::append）で行う。
pub fn cached_records<'a>(
    adj_ids: &'a [u32],
    cache:   &'a HashMap<u32, Vec<NodeRecord>>,
) -> impl Iterator<Item = &'a NodeRecord> + 'a {
    adj_ids.iter().filter_map(|&id| find_in_cache(cache, id))
}
