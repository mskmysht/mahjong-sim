use core::num;
use std::collections::BTreeMap;

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
fn all_possible_nums(num_tiles: usize, num_variations: usize) -> Vec<Vec<(u8, usize)>> {
    assert!(num_tiles <= MAX_NUM_HAND_TILES);
    let mut combinations = Vec::new();

    let n4 = num_tiles;
    let m4 = num_variations;
    for i4 in 0..(n4 / 4 + 1) {
        let n3 = n4 - i4 * 4;
        let m3 = m4 - i4;
        let mut g3 = Vec::new();
        if i4 > 0 {
            g3.push((4, i4));
        }
        let g3 = g3;
        for i3 in 0..(n3 / 3 + 1) {
            let n2 = n3 - i3 * 3;
            let m2 = m3 - i3;
            let mut g2 = g3.clone();
            if i3 > 0 {
                g2.push((4, i4));
            };
            let g2 = g2;
            for i2 in 0..(n2 / 2 + 1) {
                let n1 = n2 - i2 * 2;
                let m1 = m2 - i2;
                if m1 < n1 {
                    continue;
                }
                let mut g1 = g2.clone();
                if i2 > 0 {
                    g1.push((2, i2));
                };
                if n1 > 0 {
                    g1.push((1, n1));
                }
                combinations.push(g1);
            }
        }
    }

    combinations
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
            let (ks, iters): (Vec<_>, Vec<_>) = comb.into_iter().map(|(k, n)| {
                let iter = (0..m).combinations(n);
                m -= n;
                (k, iter)
            }).unzip();

            for jss in iters.into_iter().multi_cartesian_product() {
                let mut g = vec![0; MAX_SUHAI_NUM];
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
                update_mentsu_count(&mut g, &mut mentsu_table);
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

fn find_khotsu_patterns() -> Vec<usize> {
    todo!()
}

fn find_shuntsu_patterns() -> Vec<usize> {
    todo!()
}

fn update_mentsu_count(g: &mut Vec<u8>, mentsu_table: &mut BTreeMap<u32, i32>) {
    let c = encode_tile_nums(&g);
    let mut kim = None;
    for i in find_khotsu_patterns() {
        g[i] -= 3;
        let d = encode_tile_nums(&g);
        let temp = mentsu_table[&d] + 1;
        match kim {
            None => {
                kim = Some((i, temp));
            }
            Some((_, km)) if temp > km => {
                kim = Some((i, temp));
            }
            _ => {}
        }
        g[i] += 3;
    }
    let kim = kim;
    let mut sim = None;
    for i in find_shuntsu_patterns() {
        g[i] -= 1;
        g[i+1] -= 1;
        g[i+2] -= 1;
        let d = encode_tile_nums(&g);
        let temp = mentsu_table[&d] + 1;
        match sim {
            None => {
                sim = Some((i, temp));
            }
            Some((_, sm)) if temp > sm => {
                sim = Some((i, temp));
            }
            _ => {}
        }
        g[i] += 1;
        g[i+1] += 1;
        g[i+2] += 1;
    }
    let sim = sim;

    let e = mentsu_table.entry(c);
    match (kim, sim) {
        (None, None) => { e.or_default(); }
        (Some((i, m)), None) => {
            e.insert_entry(m);
            g[i] -= 3;
        }
        (None, Some((i, m))) => {
            e.insert_entry(m);
            g[i] -= 1;
            g[i+1] -= 1;
            g[i+2] -= 1;
        }
        (Some((ki, km)), Some((si, sm))) => {
            if km > sm {
                e.insert_entry(km);
                g[ki] -= 3;
            } else {
                e.insert_entry(sm);
                g[si] -= 1;
                g[si+1] -= 1;
                g[si+2] -= 1;
            }
        }
    }
}

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


// def find_shuntsu_patterns(g: list[int]) -> list[int]:
//   patterns = []
//   for i in range(9 - 2):
//     if g[i] > 0 and g[i + 1] > 0 and g[i + 2] > 0:
//       patterns.append(i)
//   return patterns


// def find_kohtsu_patterns(g: list[int]) -> list[int]:
//   patterns = []
//   for i in range(9):
//     if g[i] >= 3:
//       patterns.append(i)
//   return patterns


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