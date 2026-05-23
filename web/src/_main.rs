use std::collections::{BTreeMap, BTreeSet};

use gloo_net::http::Request;
use rkyv::{rancor::Error, rend::u32_le};
use util::{ArchivedNode, ArchivedShardMap, NUM_SHARD};
use yew::prelude::*;

#[derive(Default, Clone, PartialEq)]
pub struct AppState {
    pub shard_bytes: BTreeMap<u32, Vec<u8>>,

    pub expanded_children: BTreeSet<u32>,
    pub expanded_parents: BTreeSet<u32>,
}

fn main() {
    // yew::Renderer::<App>::new().render();
    yew::Renderer::<web::App>::new().render();
}

// fn app() -> Html {
//     // let records = use_state(|| Vec::<Record>::new());

//     {
//         // let records = records.clone();
//         // use_effect_with((), move |_| {
//         //     let records = records.clone();
//         //     wasm_bindgen_futures::spawn_local(async move {
//         //         let fetched_csv: String = Request::get("./assets/data.csv")
//         //             .send()
//         //             .await
//         //             .unwrap()
//         //             .text()
//         //             .await
//         //             .unwrap();
//         //         let mut rdr = csv::Reader::from_reader(Cursor::new(fetched_csv));
//         //         let mut new_records = Vec::new();
//         //         for res in rdr.deserialize() {
//         //             let record: Record = res.unwrap();
//         //             new_records.push(record);
//         //         }
//         //         records.set(new_records);
//         //     });
//         //     || ()
//         // });
//     }
//     html! {
//         <>
//         <h1>{ "Hello World" }</h1>
//         <div>
//             <h1>{"CSV Data"}</h1>
//             // <ul>
//             //     {for records.iter().map(|r| html! {
//             //         <li>{format!("{} - {}", r.id, r.value)}</li>
//             //     })}
//             // </ul>
//         </div>
//         </>
//     }
// }

#[component(App)]
pub fn app() -> Html {
    let state = use_state(AppState::default);

    // ユーザーが入力を確定したターゲットID（初期値として仮に 15626 を設定）
    let target_id = use_state(|| 1_u32);
    let input_value = use_state(String::new); // テキストボックスの入力管理

    // 入力IDが変更されたとき、必要なシャードを自動でプリロードする副作用（Effect）
    {
        let state = state.clone();
        let target_id = *target_id;
        use_effect_with(target_id, move |_| {
            let shard_id = target_id / NUM_SHARD;

            // すでにバイトデータを持っていない場合のみFetch
            if !state.shard_bytes.contains_key(&shard_id) {
                wasm_bindgen_futures::spawn_local(async move {
                    let url = format!("./assets/shards/shard_{}.bin", shard_id);
                    if let Ok(response) = Request::get(&url).send().await {
                        if let Ok(bytes) = response.binary().await {
                            let mut new_state = (*state).clone();
                            new_state.shard_bytes.insert(shard_id, bytes);

                            // ターゲットノード自体を初期状態で「上下ともに展開」しておく
                            new_state.expanded_children.insert(target_id);
                            new_state.expanded_parents.insert(target_id);

                            state.set(new_state);
                        }
                    }
                });
            } else {
                // すでにキャッシュにある場合は、展開フラグのみ更新
                let mut new_state = (*state).clone();
                new_state.expanded_children.insert(target_id);
                new_state.expanded_parents.insert(target_id);
                state.set(new_state);
            }
            || ()
        });
    }

    // 検索ボタンクリック時のハンドラ
    let on_search_submit = {
        let target_id = target_id.clone();
        let input_value = input_value.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default(); // 画面リロードを防止
            if let Ok(id) = input_value.parse::<u32>() {
                target_id.set(id);
            }
        })
    };

    // テキストボックス入力時のハンドラ
    let on_input_change = {
        let input_value = input_value.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                input_value.set(target.value());
            }
        })
    };

    html! {
        <div class="app-container" style="padding: 20px; font-family: sans-serif;">
            <h1>{"DAG Tree Explorer (rkyv + Yew)"}</h1>

            /* ID入力フォーム */
            <form onsubmit={on_search_submit} style="margin-bottom: 20px; display: flex; gap: 8px;">
                <input
                    type="number"
                    placeholder="Enter Node ID (e.g. 15626)"
                    value={(*input_value).clone()}
                    oninput={on_input_change}
                    style="padding: 6px; width: 200px;"
                />
                <button type="submit" style="padding: 6px 12px; cursor: pointer;">{"起点として展開"}</button>
            </form>

            <hr style="border: 0; border-top: 1px solid #ddd; margin: 20px 0;" />

            /* ツリー表示エリア */
            <div class="tree-root-area" style="background: #fff; border: 1px solid #eee; padding: 15px; border-radius: 6px;">
                <h3>{ format!("📍 現在の起点ノード: [{}]", *target_id) }</h3>

                // 起点となるノードをTreeNodeコンポーネントに渡す
                // 検索されたノードから双方向に広げるため、directionはデフォルトのBoth（または指定）にします
                <TreeNode
                    node_id={*target_id}
                    state={state}
                    direction={Direction::Both}
                />
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub node_id: u32,
    pub state: UseStateHandle<AppState>,
    #[prop_or(Direction::Both)]
    pub direction: Direction,
}

#[derive(Clone, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Both,
}

#[component(TreeNode)]
pub fn tree_node(props: &Props) -> Html {
    let state = &props.state;
    let node_id = props.node_id;
    let shard_id = node_id / NUM_SHARD;

    // 1. キャッシュされたバイト配列から、対象ノードの「Archived」データを直接参照する
    let node_data: Option<&ArchivedNode> = state.shard_bytes.get(&shard_id).and_then(|bytes| {
        // rkyv::access でゼロコピーでキャスト
        if let Ok(archived_shard) = rkyv::access::<ArchivedShardMap, Error>(bytes) {
            // ArchivedなHashMapから直接ノードを検索。この間、コピーやアロケーションは一切ナシ！
            archived_shard.nodes.get(&u32_le::from(node_id))
        } else {
            None
        }
    });

    let is_child_expanded = state.expanded_children.contains(&node_id);
    let is_parent_expanded = state.expanded_parents.contains(&node_id);

    macro_rules! toggle_callback {
        ($expanded: ident) => {{
        let state = state.clone();
        let has_bytes = state.shard_bytes.contains_key(&shard_id);
        Callback::from(move |_| {
            let state = state.clone();
            let mut new_state = (*state).clone();
            if new_state.$expanded.contains(&node_id) {
                new_state.$expanded.remove(&node_id);
                state.set(new_state);
            } else {
                new_state.$expanded.insert(node_id);
                if !has_bytes {
                    // バイトデータがまだ無い場合のみHTTPリクエスト
                    wasm_bindgen_futures::spawn_local(async move {
                        let url = format!("./assets/shards/shard_{}.bin", shard_id);
                        if let Ok(response) = Request::get(&url).send().await {
                            if let Ok(bytes) = response.binary().await {
                                let mut updated_state = (*state).clone();
                                // 生バイトをそのままHashMapに放り込む（超高速）
                                updated_state.shard_bytes.insert(shard_id, bytes);
                                updated_state.$expanded.insert(node_id);
                                state.set(updated_state);
                            }
                        }
                    });
                } else {
                    state.set(new_state);
                }
            }
        })
        }};
    }

    // 上位（親）の展開ボタンが押されたとき
    let toggle_parents = toggle_callback!(expanded_parents);

    // 下位（子）の展開ボタンが押されたとき
    let toggle_children = toggle_callback!(expanded_children);

    html! {
        <div class="node-wrapper" style="margin: 5px 0; font-family: monospace;">

            /* 1. 上位（親）ノードのレンダリング領域 */
            { if is_parent_expanded && (props.direction == Direction::Both || props.direction == Direction::Up) {
                if let Some(ref n) = node_data {
                    html! {
                        <div class="parents-area" style="border-left: 1px dotted #aaa; margin-left: 10px; background: #fafafa;">
                            { for n.from.iter().map(|&pid| html! {
                                // 親方向への展開は Direction::Up を強制して下方向への無限逆流を防ぐ
                                <TreeNode node_id={pid.to_native()} state={props.state.clone()} direction={Direction::Up} />
                            }) }
                        </div>
                    }
                } else { html! { <div class="loading">{"⏳ Loading Parents..."}</div> } }
            } else { html! {} } }

            /* 2. カレントノード自体の表示・操作バー */
            <div class="node-core-ui" style="display: flex; align-items: center; gap: 8px; padding: 4px; background: #eee; border-radius: 4px;">
                // 上位展開トリガー
                { if props.direction == Direction::Both || props.direction == Direction::Up {
                    html! { <button onclick={toggle_parents}>{ if is_parent_expanded { "▲ 閉じる" } else { "△ 上位展開" } }</button> }
                } else { html!{} } }

                // ノード情報
                <span class="node-info">
                    <strong>{ format!("[{}]", node_id) }</strong>
                    { if let Some(ref n) = node_data { format!(" ➡️ {:?}", rkyv::deserialize::<_, Error>(&n.data).unwrap()) } else { "".to_string() } }
                </span>

                // 下位展開トリガー
                { if props.direction == Direction::Both || props.direction == Direction::Down {
                    html! { <button onclick={toggle_children}>{ if is_child_expanded { "▼ 閉じる" } else { "▽ 下位展開" } }</button> }
                } else { html!{} } }
            </div>

            /* 3. 下位（子）ノードのレンダリング領域 */
            { if is_child_expanded && (props.direction == Direction::Both || props.direction == Direction::Down) {
                if let Some(ref n) = node_data {
                    html! {
                        <div class="children-area" style="border-left: 1px dashed #aaa; margin-left: 10px;">
                            { for n.to.iter().map(|&cid| html! {
                                // 子方向への展開は Direction::Down を強制して上方向への無限逆流を防ぐ
                                <TreeNode node_id={cid.to_native()} state={props.state.clone()} direction={Direction::Down} />
                            }) }
                        </div>
                    }
                } else { html! { <div class="loading">{"⏳ Loading Children..."}</div> } }
            } else { html! {} } }

        </div>
    }
}
