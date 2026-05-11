use std::io::Cursor;

use gloo_net::http::Request;
use serde::Deserialize;
use yew::prelude::*;

#[derive(Debug, Deserialize, Clone, PartialEq)]
struct Record {
    id: u32,
    value: u32,
}

#[component(App)]
fn app() -> Html {
    let records = use_state(|| Vec::<Record>::new());

    {
        let records = records.clone();
        use_effect_with((), move |_| {
            let records = records.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let fetched_csv: String = Request::get("./assets/data.csv")
                    .send()
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap();
                let mut rdr = csv::Reader::from_reader(Cursor::new(fetched_csv));
                let mut new_records = Vec::new();
                for res in rdr.deserialize() {
                    let record: Record = res.unwrap();
                    new_records.push(record);
                }
                records.set(new_records);
            });
            || ()
        });
    }
    html! {
        <>
        <h1>{ "Hello World" }</h1>
        <div>
            <h1>{"CSV Data"}</h1>
            <ul>
                {for records.iter().map(|r| html! {
                    <li>{format!("{} - {}", r.id, r.value)}</li>
                })}
            </ul>
        </div>
        </>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
