use core::num;
use std::{
    collections::BTreeMap,
    ops::{Index, IndexMut},
};

use itertools::Itertools;

const CODE_RADIX: u32 = 5;
const MAX_NUM_HAND_TILES: usize = 14;
const MAX_SUHAI_NUM: usize = 9;
const MAX_JIHAI_NUM: usize = 7;

fn encode_tile_nums(nums: &[u8]) -> u32 {
    let mut c = 0;
    for (i, &n) in nums.iter().enumerate() {
        c += (n as u32) * CODE_RADIX.pow(i as u32);
    }
    c
}

fn decode_to_tile_nums(mut code: u32) -> Vec<u8> {
    let mut nums = Vec::new();
    while code > 0 {
        let d = code % CODE_RADIX;
        nums.push(d as u8);
        code /= CODE_RADIX;
    }
    nums
}

fn rec_all_possible_nums(num_tiles: usize, num_tile_kinds: usize) -> Vec<Vec<(u8, usize)>> {
    assert!(num_tiles <= MAX_NUM_HAND_TILES);
    let mut combinations = Vec::new();
    let mut stack = Vec::new();

    macro_rules! generate {
        ($k:expr, $n:expr, $m:expr, $g:expr) => {
            stack.push(($k, $n, $m, $g, (0..=$n).step_by($k as usize).enumerate()));
        };
    }
    generate!(4u8, num_tiles, num_tile_kinds, Vec::new());
    loop {
        let Some((k, n, m, g, iter)) = stack.last_mut() else {
            break;
        };
        let k = *k;
        if let Some((i, l)) = iter.next() {
            let mut g = g.clone();
            if i > 0 {
                g.push((k, i));
            }
            let n = *n - l;
            let m = *m - i;
            if k > 1 {
                generate!(k - 1, n, m, g);
            } else if m >= n {
                g.push((1, n));
                combinations.push(g);
            }
        } else {
            stack.pop();
        }
    }
    combinations
}

fn all_possible_nums(num_tiles: usize, num_variations: usize) -> Vec<Vec<(u8, usize)>> {
    assert!(num_tiles <= MAX_NUM_HAND_TILES);
    let mut combinations = Vec::new();
    let num_t = num_tiles;
    let num_v = num_variations;
    for (i, l) in (0..=num_t).step_by(4).enumerate() {
        let mut g = Vec::new();
        if i > 0 {
            g.push((4, i));
        }
        let g = g;
        let num_t = num_t - l;
        let num_v = num_v - i;
        for (i, l) in (0..=num_t).step_by(3).enumerate() {
            let mut g = g.clone();
            if i > 0 {
                g.push((3, i));
            };
            let g = g;
            let num_t = num_t - l;
            let num_v = num_v - i;
            for (i, l) in (0..=num_t).step_by(2).enumerate() {
                // num_t - l > num_v - i => num_t > num_v + i
                if num_t > num_v + i {
                    continue;
                }
                let mut g = g.clone();
                if i > 0 {
                    g.push((2, i));
                };
                if num_t > l {
                    g.push((1, num_t - l));
                }
                combinations.push(g);
            }
        }
    }
    combinations
}

#[derive(Clone)]
enum GroupType {
    Triple,
    Double,
    Skip,
    Sequence,
    Continuous,
}

struct Group(GroupType, usize);

impl Group {
    fn find<const L: usize>(gt: GroupType, counter: &Counter<L>, i: usize) -> Option<Self> {
        match gt {
            GroupType::Triple => {
                let &c = counter.0.get(i)?;
                if c >= 3 { Some(Self(gt, i)) } else { None }
            }
            GroupType::Double => {
                let &c = counter.0.get(i)?;
                if c >= 2 { Some(Self(gt, i)) } else { None }
            }
            GroupType::Skip => {
                let &c0 = counter.0.get(i)?;
                let &c2 = counter.0.get(i + 2)?;
                if c0 > 0 && c2 > 0 {
                    Some(Self(gt, i))
                } else {
                    None
                }
            }
            GroupType::Sequence => {
                let &c0 = counter.0.get(i)?;
                let &c1 = counter.0.get(i + 1)?;
                if c0 > 0 && c1 > 0 {
                    Some(Self(gt, i))
                } else {
                    None
                }
            }
            GroupType::Continuous => {
                let &c0 = counter.0.get(i)?;
                let &c1 = counter.0.get(i + 1)?;
                let &c2 = counter.0.get(i + 2)?;
                if c0 > 0 && c1 > 0 && c2 > 0 {
                    Some(Self(gt, i))
                } else {
                    None
                }
            }
        }
    }
}

struct Counter<const L: usize>([u8; L]);

impl<const L: usize> Counter<L> {
    fn zero() -> Self {
        Self([0; L])
    }

    fn encode(&self) -> u32 {
        encode_tile_nums(&self.0)
    }

    fn discount(&mut self, Group(gt, i): &Group) {
        let i = *i;
        match gt {
            GroupType::Triple => {
                self.0[i] -= 3;
            }
            GroupType::Double => {
                self.0[i] -= 2;
            }
            GroupType::Skip => {
                self.0[i] -= 1;
                self.0[i + 2] -= 1;
            }
            GroupType::Sequence => {
                self.0[i] -= 1;
                self.0[i + 1] -= 1;
            }
            GroupType::Continuous => {
                self.0[i] -= 1;
                self.0[i + 1] -= 1;
                self.0[i + 2] -= 1;
            }
        }
    }

    fn count(&mut self, Group(gt, i): &Group) {
        let i = *i;
        match gt {
            GroupType::Triple => {
                self.0[i] += 3;
            }
            GroupType::Double => {
                self.0[i] += 2;
            }
            GroupType::Skip => {
                self.0[i] += 1;
                self.0[i + 2] += 1;
            }
            GroupType::Sequence => {
                self.0[i] += 1;
                self.0[i + 1] += 1;
            }
            GroupType::Continuous => {
                self.0[i] += 1;
                self.0[i + 1] += 1;
                self.0[i + 2] += 1;
            }
        }
    }

    fn find(&self, gt: GroupType) -> Vec<Group> {
        (0..L)
            .filter_map(|i| Group::find(gt.clone(), self, i))
            .collect()
    }

    fn find_max_group<T: for<'d> Index<&'d u32, Output = i32>>(
        &mut self,
        groups: Vec<Group>,
        mut mg: Option<(Group, i32)>,
        mentsu_table: &T,
    ) -> Option<(Group, i32)> {
        for m in groups {
            self.discount(&m);
            let d = self.encode();
            self.count(&m);
            let temp = mentsu_table[&d] + 1;
            if mg.is_none() {
                mg = Some((m, temp));
            } else if let Some((_, c)) = mg
                && c < temp
            {
                mg = Some((m, temp));
            }
        }
        mg
    }

    fn new(jss: Vec<Vec<usize>>, ks: &[u8]) -> Self {
        let mut counter = Self::zero();
        let mut idxs: Vec<_> = (0..9).collect();
        let mut tile_flags = [true; L];

        let jss_len = jss.len();

        for (l, js) in jss.into_iter().enumerate() {
            let k = ks[l];
            for &j in &js {
                counter.0[idxs[j]] = k;
            }
            if l == jss_len - 1 {
                break;
            }
            for j in js {
                tile_flags[idxs[j]] = false;
            }
            idxs.clear();
            for i in 0..L {
                if tile_flags[i] {
                    idxs.push(i);
                }
            }
        }
        counter
    }
}

enum Mentsu<const L: usize> {
    Khostu(usize),
    Shuntsu(usize),
}

impl<const L: usize> Mentsu<L> {
    #[inline]
    fn discount(&self, counter: &mut [u8; L]) {
        match self {
            &Mentsu::Khostu(i) => {
                counter[i] -= 3;
            }
            &Mentsu::Shuntsu(i) => {
                counter[i] -= 1;
                counter[i + 1] -= 1;
                counter[i + 2] -= 1;
            }
        }
    }

    #[inline]
    fn count(&self, counter: &mut [u8; L]) {
        match self {
            &Mentsu::Khostu(i) => {
                counter[i] += 3;
            }
            &Mentsu::Shuntsu(i) => {
                counter[i] += 1;
                counter[i + 1] += 1;
                counter[i + 2] += 1;
            }
        }
    }
}

pub struct SuhaiCounter(Counter<MAX_SUHAI_NUM>);
pub struct WindCounter(Counter<MAX_JIHAI_NUM>);

impl SuhaiCounter {
    fn update_mentsu_count(&mut self, mentsu_table: &mut BTreeMap<u32, i32>) {
        let code = self.0.encode();
        let mut mc = None;
        mc = self.0.find_max_group(self.0.find(GroupType::Triple), mc, mentsu_table);
        mc = self.0.find_max_group(self.0.find(GroupType::Continuous), mc, mentsu_table);
        mentsu_table.insert(
            code,
            mc.map(|(m, c)| {
                self.0.discount(&m);
                c
            })
            .unwrap_or_default(),
        );
    }
}

fn find_suhai_patterns() {
    let mut mentsu_table = BTreeMap::new();
    let mut tahtsu_table = BTreeMap::new();

    mentsu_table.insert(0, 0);
    tahtsu_table.insert(0, 0);

    for n in 0..=MAX_NUM_HAND_TILES {
        for comb in all_possible_nums(n, MAX_SUHAI_NUM) {
            let mut m = MAX_SUHAI_NUM;
            // let mut ks = Vec::new();
            let (ks, iters): (Vec<_>, Vec<_>) = comb
                .into_iter()
                .map(|(k, n)| {
                    let iter = (0..m).combinations(n);
                    m -= n;
                    (k, iter)
                })
                .unzip();

            for jss in iters.into_iter().multi_cartesian_product() {
                let mut counter = SuhaiCounter(Counter::new(jss, &ks));
                counter.update_mentsu_count(&mut mentsu_table);
            }
        }
    }
    //   # print(mentsu_table)
    //   # print(tahtsu_table)
    //   # for c, n in mentsu_table.items():
    //   #   g = decode(c)
    //   #   print(n, tahtsu_table[c], g)
    //   print(len(mentsu_table))
}

// def find_kohtsu_patterns(g: list[int]) -> list[int]:
//   patterns = []
//   for i in range(9):
//     if g[i] >= 3:
//       patterns.append(i)
//   return patterns

//         max_n_tahtsu = 0
//         for i in find_kanchan_patterns(g):
//           g[i] -= 1
//           g[i+2] -= 1
//           d = encode(g)
//           max_n_tahtsu = max(max_n_tahtsu, tahtsu_table[d] + 1)
//           g[i] += 1
//           g[i+2] += 1

//         for i in find_penchan_patterns(g):
//           g[i] -= 1
//           g[i+1] -= 1
//           d = encode(g)
//           max_n_tahtsu = max(max_n_tahtsu, tahtsu_table[d] + 1)
//           g[i] += 1
//           g[i+1] += 1

//         for i in find_ryanmen_patterns(g):
//           g[i+1] -= 1
//           g[i+2] -= 1
//           d = encode(g)
//           max_n_tahtsu = max(max_n_tahtsu, tahtsu_table[d] + 1)
//           g[i+1] += 1
//           g[i+2] += 1

//         for i in find_toitsu_patterns(g):
//           g[i] -= 2
//           d = encode(g)
//           max_n_tahtsu = max(max_n_tahtsu, tahtsu_table[d] + 1)
//           g[i] += 2
//         tahtsu_table.setdefault(c, 0)
//         tahtsu_table[c] = max_n_tahtsu

// def main():
