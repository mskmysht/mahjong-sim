use std::{
    collections::BTreeMap,
    ops::{Index, IndexMut},
};

use itertools::Itertools;

const CODE_RADIX: u32 = 5;
const MAX_NUM_HAND_TILES: usize = 14;
const NUM_NUMBER_TILES: usize = 9;
const NUM_CHAR_TILES: usize = 7;

fn encode_tile_nums(nums: &[u8]) -> u32 {
    let mut c = 0;
    for (i, &n) in nums.iter().enumerate() {
        c += (n as u32) * CODE_RADIX.pow(i as u32);
    }
    c
}

fn decode_to_tile_nums<const L: usize>(mut code: u32) -> [u8; L] {
    let mut nums = [0; L];
    for i in 0..L {
        nums[i] = (code % CODE_RADIX) as u8;
        code /= CODE_RADIX;
    }
    nums
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
pub enum GroupType {
    Triple,
    Sequence,
    Neighbor,
    Skip,
    Double,
}

struct Group(GroupType, usize);

impl Group {
    fn find<const L: usize>(gt: GroupType, counter: &TileCounter<L>, i: usize) -> Option<Self> {
        match gt {
            GroupType::Triple => {
                let &c = counter.0.get(i)?;
                if c >= 3 { Some(Self(gt, i)) } else { None }
            }
            GroupType::Double => {
                let &c = counter.0.get(i)?;
                if c >= 2 { Some(Self(gt, i)) } else { None }
            }
            GroupType::Sequence => {
                let &c0 = counter.0.get(i)?;
                let &c1 = counter.0.get(i + 1)?;
                let &c2 = counter.0.get(i + 2)?;
                if c0 > 0 && c1 > 0 && c2 > 0 {
                    Some(Self(gt, i))
                } else {
                    None
                }
            }
            GroupType::Neighbor => {
                let &c0 = counter.0.get(i)?;
                let &c1 = counter.0.get(i + 1)?;
                if c0 > 0 && c1 > 0 {
                    Some(Self(gt, i))
                } else {
                    None
                }
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
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct GroupCounter {
    inner: Vec<u32>,
}

impl GroupCounter {
    fn new() -> Self {
        Self { inner: Vec::new() }
    }
    #[inline]
    fn to_index(gt: &GroupType) -> usize {
        match gt {
            GroupType::Triple => 0,
            GroupType::Sequence => 1,
            GroupType::Double => 2,
            GroupType::Skip => 3,
            GroupType::Neighbor => 4,
        }
    }

    fn all(&self) -> u32 {
        self.inner.iter().sum()
    }
}

impl Index<&GroupType> for GroupCounter {
    type Output = u32;
    fn index(&self, index: &GroupType) -> &Self::Output {
        let i = Self::to_index(index);
        self.inner.get(i).unwrap_or(&0)
    }
}

impl IndexMut<&GroupType> for GroupCounter {
    fn index_mut(&mut self, index: &GroupType) -> &mut Self::Output {
        let i = Self::to_index(index);
        for _ in (self.inner.len())..=i {
            self.inner.push(0);
        }
        &mut self.inner[Self::to_index(index)]
    }
}

pub struct TileGroupMap {
    inner: BTreeMap<u32, GroupCounter>,
}

impl<'a> IntoIterator for &'a TileGroupMap {
    type Item = <&'a BTreeMap<u32, GroupCounter> as IntoIterator>::Item;
    type IntoIter = <&'a BTreeMap<u32, GroupCounter> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.inner).into_iter()
    }
}

#[derive(Debug)]
pub struct TileCounter<const L: usize>([u8; L]);

impl<const L: usize> TileCounter<L> {
    fn zero() -> Self {
        Self([0; L])
    }

    pub fn encode(&self) -> u32 {
        encode_tile_nums(&self.0)
    }

    pub fn decode(code: u32) -> Self {
        Self(decode_to_tile_nums(code))
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
            GroupType::Neighbor => {
                self.0[i] -= 1;
                self.0[i + 1] -= 1;
            }
            GroupType::Sequence => {
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
            GroupType::Neighbor => {
                self.0[i] += 1;
                self.0[i + 1] += 1;
            }
            GroupType::Sequence => {
                self.0[i] += 1;
                self.0[i + 1] += 1;
                self.0[i + 2] += 1;
            }
        }
    }

    fn find(&self, gt: &GroupType) -> Vec<Group> {
        (0..L)
            .filter_map(|i| Group::find(gt.clone(), self, i))
            .collect()
    }

    fn insert_group_count(&mut self, target_gts: &[GroupType], tgm: &mut TileGroupMap) {
        let code = self.encode();
        let mut mg = None;
        for gt in target_gts {
            for group in self.find(gt) {
                self.discount(&group);
                let d = self.encode();
                self.count(&group);
                let gc = &tgm.inner[&d];
                let temp = gc.all();
                if mg.is_none() {
                    mg = Some((group, gc, temp));
                } else if let Some((_, _, a)) = mg
                    && a < temp
                {
                    mg = Some((group, gc, temp));
                }
            }
        }
        if let Some((group, gc, _)) = mg.take() {
            self.discount(&group);
            let mut gc = gc.clone();
            gc[&group.0] += 1;
            // assert!(!tgm.inner.contains_key(&code), "{:?}", Self::decode(code));
            tgm.inner.insert(code, gc);
        } else {
            tgm.inner.insert(code, GroupCounter::new());
        }
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

impl TileGroupMap {
    fn empty() -> Self {
        let mut inner = BTreeMap::new();
        inner.insert(0, GroupCounter::new());
        TileGroupMap { inner }
    }
}

pub struct TileGroupData {
    pub compl_map: TileGroupMap,
    pub imcmp_map: TileGroupMap,
}

impl TileGroupData {
    fn new<const L: usize>(complete_gts: &[GroupType], impcomplete_gts: &[GroupType]) -> Self {
        let mut compl_map = TileGroupMap::empty();
        let mut imcmp_map = TileGroupMap::empty();

        for n in 0..=6 {
            for comb in all_possible_nums(n, L) {
                let mut m = L;
                let (ks, iters): (Vec<_>, Vec<_>) = comb
                    .into_iter()
                    .map(|(k, n)| {
                        let iter = (0..m).combinations(n);
                        m -= n;
                        (k, iter)
                    })
                    .unzip();

                for jss in iters.into_iter().multi_cartesian_product() {
                    let mut counter = TileCounter::<L>::new(jss, &ks);
                    counter.insert_group_count(complete_gts, &mut compl_map);
                    counter.insert_group_count(impcomplete_gts, &mut imcmp_map);
                }
            }
        }
        Self {
            compl_map,
            imcmp_map,
        }
    }
}

pub type NumberTileCounter = TileCounter<NUM_NUMBER_TILES>;
pub type CharTileCounter = TileCounter<NUM_CHAR_TILES>;

pub fn number_tiles_data() -> TileGroupData {
    TileGroupData::new::<NUM_NUMBER_TILES>(
        &[GroupType::Triple, GroupType::Sequence],
        &[GroupType::Double, GroupType::Skip, GroupType::Neighbor],
    )
}

pub fn char_tiles_data() -> TileGroupData {
    TileGroupData::new::<NUM_CHAR_TILES>(&[GroupType::Triple], &[GroupType::Double])
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
