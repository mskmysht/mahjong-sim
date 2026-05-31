use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Write,
    ops::{Index, IndexMut},
    path::PathBuf,
};

use itertools::Itertools;
use rkyv::{
    Serialize, api::high::HighSerializer, rancor::Error, ser::allocator::ArenaHandle,
    util::AlignedVec,
};

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

fn decode_to_tile_vec(mut code: u32) -> Vec<u32> {
    let mut tiles = Vec::new();
    let mut i = 0;
    while code > 0 {
        for _ in 0..(code % CODE_RADIX) {
            tiles.push(i);
        }
        code /= CODE_RADIX;
        i += 1;
    }
    tiles
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

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct GroupCounter {
    inner: [u32; 5],
}

impl GroupCounter {
    fn new() -> Self {
        Self { inner: [0; 5] }
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
}

impl Index<&GroupType> for GroupCounter {
    type Output = u32;
    fn index(&self, index: &GroupType) -> &Self::Output {
        &self.inner[Self::to_index(index)]
    }
}

impl IndexMut<&GroupType> for GroupCounter {
    fn index_mut(&mut self, index: &GroupType) -> &mut Self::Output {
        &mut self.inner[Self::to_index(index)]
    }
}

pub struct TileGroupMap {
    pub counter: BTreeMap<u32, GroupCounter>,
    pub succs: BTreeMap<u32, Vec<u32>>,
    pub preds: BTreeMap<u32, Vec<u32>>,
}

struct Group(GroupType, usize);

impl Group {
    fn to_counter<const L: usize>(&self) -> Option<TileCounter<L>> {
        let i = self.1;
        let mut counter = TileCounter::zero();
        match self.0 {
            GroupType::Triple => {
                if i >= L {
                    return None;
                }
                counter.0[i] = 3;
            }
            GroupType::Double => {
                if i >= L {
                    return None;
                }
                counter.0[i] = 2;
            }
            GroupType::Skip => {
                if i + 2 >= L {
                    return None;
                }
                counter.0[i] = 1;
                counter.0[i + 2] = 1;
            }
            GroupType::Neighbor => {
                if i + 1 >= L {
                    return None;
                }
                counter.0[i] = 1;
                counter.0[i + 1] = 1;
            }
            GroupType::Sequence => {
                if i + 2 >= L {
                    return None;
                }
                counter.0[i] = 1;
                counter.0[i + 1] = 1;
                counter.0[i + 2] = 1;
            }
        }
        Some(counter)
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

    fn ge(&self, other: &Self) -> bool {
        let mut p = true;
        for i in 0..L {
            if self.0[i] < other.0[i] {
                p = false;
                break;
            }
        }
        p
    }

    fn find_opt_group(
        &self,
        target_gts: &[GroupType],
        tgm: &TileGroupMap,
    ) -> Option<(Group, Self, u32, u32)> {
        let code = self.encode();
        let mut temp = None;
        for gt in target_gts {
            for i in 0..L {
                let group = Group(gt.clone(), i);
                let Some(part) = group.to_counter() else {
                    continue;
                };
                if !self.ge(&part) {
                    continue;
                }
                let d = code - part.encode();
                let gc = &tgm.counter[&d];
                let sum = target_gts.iter().map(|gt| gc[gt]).sum();
                if let Some((_, _, _, a)) = temp
                    && a >= sum
                {
                    continue;
                }
                temp = Some((group, part, d, sum));
            }
        }
        temp
    }

    fn from_comb(jss: Vec<Vec<usize>>, ks: &[u8]) -> Self {
        let mut counter = Self::zero();
        let mut idxs: Vec<_> = (0..L).collect();
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
        let mut counter = BTreeMap::new();
        let preds = BTreeMap::new();
        let succs = BTreeMap::new();
        counter.insert(0, GroupCounter::new());
        TileGroupMap {
            counter,
            preds,
            succs,
        }
    }

    fn update(&mut self, code: u32, prev: u32, prev_group: Group) {
        let mut gc = self.counter[&prev].clone();
        gc[&prev_group.0] += 1;
        self.counter.insert(code, gc);
        self.preds.entry(code).or_default().push(prev);
        self.succs.entry(prev).or_default().push(code);
    }
}

pub struct TileGroupData {
    pub compl_map: TileGroupMap,
    pub imcmp_map: TileGroupMap,
}

impl TileGroupData {
    fn new<const L: usize>(
        num_tiles: usize,
        complete_gts: &[GroupType],
        impcomplete_gts: &[GroupType],
    ) -> Self {
        let mut compl_map = TileGroupMap::empty();
        let imcmp_map = TileGroupMap::empty();

        for n in 0..=num_tiles {
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
                    let counter = TileCounter::<L>::from_comb(jss, &ks);
                    let code = counter.encode();
                    if let Some((prev_group, _, prev, _)) =
                        counter.find_opt_group(complete_gts, &compl_map)
                    {
                        compl_map.update(code, prev, prev_group);
                    } else if let Some((prev_group, _, prev, _)) =
                        counter.find_opt_group(impcomplete_gts, &compl_map)
                    {
                        compl_map.update(code, prev, prev_group);
                    } else {
                        compl_map.counter.insert(code, GroupCounter::new());
                    }
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

pub fn number_tiles_data(num_tiles: usize) -> TileGroupData {
    TileGroupData::new::<NUM_NUMBER_TILES>(
        num_tiles,
        &[GroupType::Triple, GroupType::Sequence],
        &[GroupType::Double, GroupType::Skip, GroupType::Neighbor],
    )
}

pub fn char_tiles_data() -> TileGroupData {
    TileGroupData::new::<NUM_CHAR_TILES>(6, &[GroupType::Triple], &[GroupType::Double])
}

impl From<TileGroupData> for BTreeMap<u32, util::ShardMap> {
    fn from(mut value: TileGroupData) -> Self {
        let mut shards: BTreeMap<u32, util::ShardMap> = BTreeMap::new();
        for (id, gc) in value.compl_map.counter {
            let shard_id = id / util::NUM_SHARD;
            let shard = shards.entry(shard_id).or_default();
            let label = decode_to_tile_vec(id).iter().join(",");
            shard.nodes.insert(
                id,
                util::Node::new(
                    value.compl_map.preds.remove(&id).unwrap_or_default(),
                    value.compl_map.succs.remove(&id).unwrap_or_default(),
                    label,
                    util::NodeScore::new(
                        gc[&GroupType::Triple],
                        gc[&GroupType::Sequence],
                        gc[&GroupType::Neighbor],
                        gc[&GroupType::Skip],
                        gc[&GroupType::Double],
                    ),
                ),
            );
        }
        shards
    }
}

impl From<TileGroupData> for BTreeMap<u32, Vec<util::NodeRecord>> {
    fn from(mut value: TileGroupData) -> Self {
        let mut shard_map: BTreeMap<u32, Vec<util::NodeRecord>> = BTreeMap::new();
        for (id, gc) in value.compl_map.counter {
            let shard_id = id / util::SHARD_SIZE;
            let recs = shard_map.entry(shard_id).or_default();
            let tiles = decode_to_tile_vec(id);
            recs.push(util::NodeRecord::new(
                id,
                tiles,
                util::NodeScore::new(
                    gc[&GroupType::Triple],
                    gc[&GroupType::Sequence],
                    gc[&GroupType::Neighbor],
                    gc[&GroupType::Skip],
                    gc[&GroupType::Double],
                ),
                 value.compl_map.preds.remove(&id).unwrap_or_default(),
                 value.compl_map.succs.remove(&id).unwrap_or_default(),
            ));
        }
        shard_map
    }
}

pub fn export_tile_data<T>(data: TileGroupData, dir: &str) -> Result<(), Box<dyn std::error::Error>>
where
    TileGroupData: Into<BTreeMap<u32, T>>,
    for<'a> T: Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, Error>>,
{
    let dir = PathBuf::from(dir);
    fs::create_dir_all(&dir)?;
    for (shard_id, shard) in data.into() {
        let bytes = rkyv::to_bytes::<Error>(&shard)?;
        let mut file = File::create(dir.join(format!("shard_{}.bin", shard_id)))?;
        file.write_all(&bytes)?;
    }

    Ok(())
}
