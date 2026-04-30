use core::num;
use std::{
    collections::BTreeMap,
    ops::{Index, IndexMut},
};

use itertools::Itertools;

const CODE_RADIX: u32 = 5;
const MAX_NUM_HAND_TILES: usize = 14;
const MAX_SUHAI_NUM: usize = 9;

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

// def iterate(n: int) -> list[list[int]]:
//   assert n <= 14
//   combs = []
//   for i in range(0, n // 4 + 1):
//     n1 = n - i * 4
//     m1 = 9 - i
//     g1 = [] if i == 0 else [(4, i)]
//     for j in range(0, n1 // 3 + 1):
//       n2 = n1 - j * 3
//       m2 = m1 - j
//       g2 = [p for p in g1]
//       if j > 0:
//         g2.append((3, j))
//       for k in range(0, n2 // 2 + 1):
//         n3 = n2 - k * 2
//         m3 = m2 - k
//         if m3 < n3:
//           continue
//         g3 = [p for p in g2]
//         if k > 0:
//           g3.append((2, k))
//         if n3 > 0:
//           g3.append((1, n3))
//         combs.append(g3)
//   return combs

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

fn find_khotsu_patterns<const L: usize>(counter: &[u8; L]) -> Vec<Mentsu<L>> {
    let mut ps = Vec::new();
    for i in 0..L {
        if counter[i] >= 3 {
            ps.push(Mentsu::Khostu(i));
        }
    }
    ps
}

// def find_shuntsu_patterns(g: list[int]) -> list[int]:
//   patterns = []
//   for i in range(9 - 2):
//     if g[i] > 0 and g[i + 1] > 0 and g[i + 2] > 0:
//       patterns.append(i)
//   return patterns
fn find_shuntsu_patterns<const L: usize>(counter: &[u8; L]) -> Vec<Mentsu<L>> {
    let mut ps = Vec::new();
    for i in 0..(L - 2) {
        if counter[i] > 0 && counter[i + 1] > 0 && counter[i + 2] > 0 {
            ps.push(Mentsu::Shuntsu(i));
        }
    }
    ps
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

#[derive(Default)]
struct SuhaiCounter([u8; MAX_SUHAI_NUM]);

impl Index<usize> for SuhaiCounter {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for SuhaiCounter {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.0.index_mut(index)
    }
}

impl SuhaiCounter {
    fn encode(&self) -> u32 {
        encode_tile_nums(&self.0)
    }

    fn find_mentsu_patterns(&self) -> Vec<Mentsu<MAX_SUHAI_NUM>> {
        let mut ps = find_khotsu_patterns(&self.0);
        ps.append(&mut find_shuntsu_patterns(&self.0));
        ps
    }

    #[inline]
    fn discount(&mut self, m: &Mentsu<MAX_SUHAI_NUM>) {
        m.discount(&mut self.0);
    }

    #[inline]
    fn count(&mut self, m: &Mentsu<MAX_SUHAI_NUM>) {
        m.count(&mut self.0);
    }

    fn update_mentsu_count(&mut self, mentsu_table: &mut BTreeMap<u32, i32>) {
        let code = self.encode();
        let mut mc = None;
        for m in self.find_mentsu_patterns() {
            self.discount(&m);
            let d = self.encode();
            self.count(&m);
            let temp = mentsu_table[&d] + 1;
            if mc.is_none() {
                mc = Some((m, temp));
            } else if let Some((_, c)) = mc
                && c < temp
            {
                mc = Some((m, temp));
            }
        }
        mentsu_table.insert(
            code,
            mc.map(|(m, c)| {
                self.discount(&m);
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
                let mut g = SuhaiCounter::default();
                let mut idxs: Vec<_> = (0..9).collect();
                let mut hai_counter = [1; MAX_SUHAI_NUM];

                let jss_len = jss.len();

                for (l, js) in jss.into_iter().enumerate() {
                    let k = ks[l];
                    for &j in &js {
                        g[idxs[j]] = k;
                    }
                    if l == jss_len - 1 {
                        break;
                    }
                    for j in js {
                        hai_counter[idxs[j]] -= 1;
                    }
                    idxs.clear();
                    for i in 0..MAX_SUHAI_NUM {
                        if hai_counter[i] > 0 {
                            idxs.push(i);
                        }
                    }
                }
                g.update_mentsu_count(&mut mentsu_table);
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

// def find_toitsu_patterns(g: list[int]) -> list[int]:
//   patterns = []
//   for i in range(9):
//     if g[i] == 2:
//       patterns.append(i)
//   return patterns

// def find_ryanmen_patterns(g: list[int]) -> list[int]:
//   patterns = []
//   for i in range(9 - 3):
//     if g[i] == 0 and g[i + 1] > 0 and g[i + 2] > 0 and g[i + 3] == 0:
//       patterns.append(i)
//   return patterns

// def find_penchan_patterns(g: list[int]) -> list[int]:
//   patterns = []
//   if g[0] > 0 and g[1] > 0 and g[2] == 0:
//     patterns.append(0)
//   if g[8] > 0 and g[7] > 0 and g[6] == 0:
//     patterns.append(7)
//   return patterns

// def find_kanchan_patterns(g: list[int]) -> list[int]:
//   patterns = []
//   for i in range(9 - 2):
//     if g[i] > 0 and g[i + 1] == 0 and g[i + 2] > 0:
//       patterns.append(i)
//   return patterns

// def main():
