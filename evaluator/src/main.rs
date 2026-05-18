use std::env::args;

use evaluator::{export_tile_data, number_tiles_data};

fn main() {
    // let mut stack = make_stack(1);
    // let mut hand = Hand {
    //     closed: stack.drain(0..13).collect(),
    //     opened: vec![],
    // };
    // hand.closed.sort();

    // for t in &hand.closed {
    //     print!("{} ", t);
    // }
    // println!("{:?}", hand.closed);
    let args = args().collect::<Vec<_>>();
    let n = args[1].parse().unwrap();

    // println!("{}", std::mem::size_of::<&Tile>());
    let data = number_tiles_data(n);
    // export_tile_data::<util::ShardMap>(data).unwrap();
    export_tile_data::<Vec<util::NodeRecord>>(data, &args[2]).unwrap();
}
