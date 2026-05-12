use evaluator::{NumberTileCounter, number_tiles_data};

use rand::{SeedableRng, rngs::SmallRng, seq::SliceRandom};

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
enum Wind {
    East,
    South,
    West,
    North,
}

impl std::fmt::Display for Wind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = match self {
            Wind::East => '\u{1F000}',
            Wind::South => '\u{1F001}',
            Wind::West => '\u{1F002}',
            Wind::North => '\u{1F003}',
        };
        write!(f, "{c}")
    }
}

// three tiles
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
enum Dragon {
    White,
    Green,
    Red,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
enum Suit {
    Man(u8),
    So(u8),
    Pin(u8),
}

impl std::fmt::Display for Dragon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = match self {
            Dragon::White => '\u{1F006}',
            Dragon::Green => '\u{1F005}',
            Dragon::Red => '\u{1F004}',
        };
        write!(f, "{c}")
    }
}

impl std::fmt::Display for Suit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let u = match self {
            Suit::Man(n) => 0x1F006 + (*n as u32),
            Suit::So(n) => 0x1F00F + (*n as u32),
            Suit::Pin(n) => 0x1F018 + (*n as u32),
        };
        write!(f, "{}", char::from_u32(u).unwrap())
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
enum Tile {
    Suited(Suit),
    Wind(Wind),
    Dragon(Dragon),
}

impl std::fmt::Display for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tile::Suited(suit) => suit.fmt(f),
            Tile::Wind(wind) => wind.fmt(f),
            Tile::Dragon(dragon) => dragon.fmt(f),
        }
    }
}

enum Group {
    Iso(Tile),  // ([A])
    Cont(Tile), // ([1], 2), ([5], 6), ([8], 9)
    Skip(Tile), // ([1], 3), ([5], 7), ([7], 9)
    Pair(Tile), // ([A], A)
    Seq(Tile),  // ([1], 2, 3)
    Tri(Tile),  // ([A], A, A)
    Quad(Tile), // ([A], A, A, A)
}

#[derive(Debug)]
struct Hand {
    // 純手牌
    closed: Vec<Tile>,
    // 副露
    opened: Vec<Tile>,
}

struct LastOne {
    tile: Tile,
    seat: Wind,
}

fn make_stack(seed: u64) -> Vec<Tile> {
    let mut complete_tiles = Vec::with_capacity((4 + 3 + 9 * 3) * 4);
    for _ in 0..4 {
        complete_tiles.push(Tile::Wind(Wind::East));
        complete_tiles.push(Tile::Wind(Wind::South));
        complete_tiles.push(Tile::Wind(Wind::West));
        complete_tiles.push(Tile::Wind(Wind::North));
        complete_tiles.push(Tile::Dragon(Dragon::White));
        complete_tiles.push(Tile::Dragon(Dragon::Green));
        complete_tiles.push(Tile::Dragon(Dragon::Red));
    }
    for _ in 0..4 {
        for n in 1..=9 {
            complete_tiles.push(Tile::Suited(Suit::Pin(n)));
        }
    }
    for _ in 0..4 {
        for n in 1..=9 {
            complete_tiles.push(Tile::Suited(Suit::So(n)));
        }
    }
    for _ in 0..4 {
        for n in 1..=9 {
            complete_tiles.push(Tile::Suited(Suit::Man(n)));
        }
    }
    let mut rng = SmallRng::seed_from_u64(seed);
    complete_tiles.shuffle(&mut rng);
    complete_tiles
}

fn cont_parse(tiles: Vec<Tile>) {}

fn parse(tiles: Vec<Tile>) -> Vec<Vec<Group>> {
    assert!(tiles.len() == 14);
    let mut all = Vec::new();
    // tiles.sort();
    let mut suits = Vec::new();
    let mut winds = Vec::new();
    let mut drgns = Vec::new();
    for tile in tiles {
        match tile {
            Tile::Suited(suit) => suits.push(suit),
            Tile::Wind(wind) => winds.push(wind),
            Tile::Dragon(dragon) => drgns.push(dragon),
        }
    }
    suits.sort();
    winds.sort();
    drgns.sort();

    // let mut groups = Vec::new();

    all
}

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

    // println!("{}", std::mem::size_of::<&Tile>());
    let data = number_tiles_data();
    let (edges, counters) = data.convert();
    util::print(edges.into_iter(), "web/assets/edges.csv").unwrap();
    util::print(counters.into_iter(), "web/assets/counters.csv").unwrap();
}
