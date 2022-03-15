use std::{
    ops::{Index, IndexMut},
    time::Instant,
};

use num::{Integer, NumCast, ToPrimitive, Unsigned};
use succinct::{
    rank::BitRankSupport, BitVec, BitVecMut, BitVector, Rank9, IntVec,
};

pub struct LS(BitVector<usize>, usize);

impl LS {
    #[inline(always)]
    pub fn new<I>(s: &[I]) -> Self
    where
        I: Unsigned + Integer,
    {
        Self(Self::calc_ls(s), s.len())
    }

    #[inline(always)]
    pub fn is_l(&self, i: usize) -> bool {
        self.0.get_bit(i as u64)
    }

    #[inline(always)]
    pub fn is_s(&self, i: usize) -> bool {
        !self.0.get_bit(i as u64)
    }

    pub fn is_lms(&self, i: usize) -> bool {
        if i == 0 {
            return false;
        }
        self.is_l(i - 1) && self.is_s(i)
    }

    pub fn len(&self) -> usize {
        self.1
    }

    //https://ls11-www.cs.tu-dortmund.de/people/rahmann/algoseq.pdf
    fn calc_ls<I>(s: &[I]) -> BitVector<usize>
    where
        I: Unsigned + Integer,
    {
        // even positions: 0 = S, 1 = L; odd positions: 1 = LMS, 0 = not LMS
        let mut ls = BitVector::with_fill(s.len() as u64, false);

        // This is for the sentinel. it is always considered to be an S value
        ls.set_bit(s.len() as u64 - 1, false);

        for i in (0..s.len() - 1).rev() {
            use std::cmp::Ordering::*;
            let ordering = I::cmp(&s[i], &s[i + 1]);
            let i = i as u64;
            match ordering {
                Greater => ls.set_bit(i, true),
                Less => ls.set_bit(i, false),
                Equal => {
                    let previous_value = ls.get_bit(i + 1);
                    ls.set_bit(i, previous_value);
                }
            }
        }

        ls
    }
}

pub fn iss<I>(s: &[I])
where
    I: Unsigned + Integer + NumCast + Copy,
{
    // We assume that all ascii numbers exist in the text, so we start counting at 256 for nonterminals
    let mut symbol_cnt = 256 as usize;
    // We can give the sentinel the same number in each iteration since it's always its own LMS-substring
    // '\0' is recommended as a sentinel
    let sentinel_n: usize = NumCast::from(s[s.len() - 1]).unwrap();

    let ls = LS::new(s);

    // naive sort ting
    let lms_pos: Vec<usize> = lms_sort(s, &ls, symbol_cnt);
}


fn naive_lms_sort<I>(s: &[I], ls: &LS) -> Vec<usize>
where
    I: Unsigned + Integer + NumCast + Copy,
{
    let lms_pos = (0..ls.len()).filter(|&i| ls.is_lms(i)).collect::<Vec<_>>();
    let n = lms_pos.len();
    let mut lms_pos_order = (0..lms_pos.len()).collect::<Vec<_>>();
    lms_pos_order.sort_by(|&a, &b| {
        let end_a = if a == n - 1 { a } else { a + 1 };
        let end_b = if b == n - 1 { b } else { b + 1 };
        s[lms_pos[a]..=lms_pos[end_a]].cmp(&s[lms_pos[b]..lms_pos[end_b]])
    });
    lms_pos_order.into_iter().map(|i| lms_pos[i]).collect()
}

fn lms_sort<I>(s: &[I], ls: &LS, symbol_cnt: usize) -> Vec<usize>
where
    I: Unsigned + Integer + ToPrimitive + Copy,
{
    //let size_bits = f64::log2(s.len() as f64 - 1.0).ceil() as usize;
    //let alphabet_bits = f64::log2(symbol_cnt as f64 - 1.0).ceil() as usize;
    let mut bucket_store = vec![usize::MAX; s.len()];

    // Calculate the buckets for the chars in the alphabet
    // The bitvector has a 1 at each index for which there is a nonzero-sized bucket. It also uses a rankDS (Vigna 2020)
    // Subsequently, bucket_start and bucket_end only save data for chars that actually appear in the text.
    let (existing_chars, mut bucket_start, mut bucket_end) = {
        let mut bucket_sizes = vec![0usize; symbol_cnt];
        let mut existing = BitVector::<u64>::with_fill(symbol_cnt as u64, false);
        for &c in s {
            bucket_sizes[c.to_usize().unwrap()] += 1;
            existing.set_bit(c.to_u64().unwrap(), true);
        }
        let (bucket_start, bucket_end) = bucket_sizes
            .into_iter()
            .filter(|&size| size > 0)
            .scan(0usize, |state, size| {
                let old_state = *state;
                *state = *state + size;
                Some((old_state, *state))
            })
            .unzip::<usize, usize, Vec<_>, Vec<_>>();
        (Rank9::new(existing), bucket_start, bucket_end)
    };

    let lms_pos = (0..ls.len()).filter(|&i| ls.is_lms(i)).collect::<Vec<_>>();
    // Schritt 0: Schreibe alle LMS Positionen an das ende ihres Blockes
    {
        // We're cloning this because we need the original later unfortunately
        let mut bucket_end = bucket_end.clone();
        // Iterate through the lms positions in reverse
        for &pos in lms_pos.iter().rev() {
            let c = s[pos].to_u64().unwrap();
            let c_pos = existing_chars.rank1(c) as usize - 1;
            bucket_store[bucket_end[c_pos] - 1] = pos;
            bucket_end[c_pos] -= 1;
        }
    }

    // Schritt a: Durchlaufe von links nach rechts. Wenn bucket_store[r] - 1 eine L position ist, schreibe die Position an die erste freie Position in ihrem Bucket
    for r in 0..bucket_store.len() {
        if bucket_store[r] == usize::MAX || bucket_store[r] == 0 {
            continue;
        }
        let pos = bucket_store[r] - 1;
        if ls.is_l(pos) {
            let c = s[pos].to_u64().unwrap();
            let c_pos = existing_chars.rank1(c) as usize - 1;
            bucket_store[bucket_start[c_pos]] = pos;
            bucket_start[c_pos] += 1;
        }
    }
    drop(bucket_start);

    //Schritt b: Durchlaufe von rechts nach links: Wenn bucket_store[r] - 1 eine S position ist, trage sie in ihrem Bucket ein
    for r in (0..bucket_store.len()).rev() {
        if bucket_store[r] == 0 {
            continue;
        }
        let pos = bucket_store[r] - 1;
        if ls.is_s(pos) {
            let c = s[pos].to_u64().unwrap();
            let c_pos = existing_chars.rank1(c) as usize - 1;
            bucket_store[bucket_end[c_pos] - 1] = pos;
            bucket_end[c_pos] -= 1;
        }
    }
    drop(bucket_end);
    drop(existing_chars);

    bucket_store
        .into_iter()
        .filter(|&pos| ls.is_lms(pos))
        .collect()
}
