use num::{cast::AsPrimitive, Integer, Unsigned};
use succinct::{
    rank::BitRankSupport, storage::BlockType, BitVec, BitVecMut, BitVector, IntVec, IntVecMut,
    IntVector, Rank9,
};

pub struct LS(BitVector<usize>, usize);

//https://ls11-www.cs.tu-dortmund.de/people/rahmann/algoseq.pdf
impl LS {
    pub fn inner(&self) -> &BitVector<usize> {
        &self.0
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
        //println!("ls[{}]: {}, ls[{}]: {}, so: {}", i-1, self.0.get_bit(i as u64 - 1), i, self.0.get_bit(i as u64), self.is_l(i - 1) && self.is_s(i));
        self.is_l(i - 1) && self.is_s(i)
    }

    pub fn next_lms_index(&self, i: usize) -> Option<usize> {
        for k in i + 1..self.len() {
            if self.is_lms(k) {
                return Some(k);
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.1
    }

    pub fn is_empty(&self) -> bool {
        self.1 == 0
    }
}

impl<I> From<&[I]> for LS
where
    I: Unsigned + Integer,
{
    fn from(s: &[I]) -> Self {
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

        Self(ls, s.len())
    }
}

impl<B: BlockType + Unsigned + Integer> From<&IntVector<B>> for LS {
    fn from(s: &IntVector<B>) -> Self {
        // even positions: 0 = S, 1 = L; odd positions: 1 = LMS, 0 = not LMS
        let mut ls = BitVector::with_fill(s.len() as u64, false);

        // This is for the sentinel. it is always considered to be an S value
        ls.set_bit(s.len() as u64 - 1, false);

        for i in (0..s.len() - 1).rev() {
            use std::cmp::Ordering::*;
            let ordering = B::cmp(&s.get(i), &s.get(i + 1));
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

        Self(ls, s.len() as usize)
    }
}

pub fn iss(vec: &IntVector<usize>, ls: &LS, max_symbol: usize) -> IntVector<usize> {
    let lms_pos_unsorted = (0..ls.len()).filter(|&i| ls.is_lms(i)).collect::<Vec<_>>();
    lms_sort(vec, ls, max_symbol, &lms_pos_unsorted)
}

fn lms_sort(
    s: &IntVector<usize>,
    ls: &LS,
    symbol_cnt: usize,
    lms_pos: &[usize],
) -> IntVector<usize> {
    let mut size_bits = f64::log2(ls.len() as f64 - 1.0).ceil() as usize;
    // You could run into problems if the max number in this bit width is contained in the input. If so, it interferes with checking for invalid values.
    if size_bits < std::mem::size_of::<usize>() * 8 {
        size_bits += 1;
    }
    //let alphabet_bits = f64::log2(symbol_cnt as f64 - 1.0).ceil() as usize;
    let invalid = !(usize::MAX << size_bits);
    let mut bucket_store = IntVector::<usize>::with_fill(size_bits, ls.len() as u64, invalid);
    // Calculate the buckets for the chars in the alphabet
    // The bitvector has a 1 at each index for which there is a nonzero-sized bucket. It also uses a rankDS (Vigna 2020)
    // Subsequently, bucket_start and bucket_end only save data for chars that actually appear in the text.
    let (existing_chars, mut bucket_start, mut bucket_end) = bucket_info(s, symbol_cnt);
    // Schritt 0: Schreibe alle LMS Positionen an das ende ihres Blockes
    {
        // We're cloning this because we need the original later unfortunately
        let mut bucket_end = bucket_end.clone();
        // Iterate through the lms positions in reverse
        for &pos in lms_pos.iter().rev() {
            let c = s.get(pos as u64).as_();
            let c_pos = existing_chars.rank1(c) - 1;
            bucket_store.set(bucket_end.get(c_pos) as u64 - 1, pos);
            bucket_end.set(c_pos, bucket_end.get(c_pos) - 1);
        }
    }

    // Schritt a: Durchlaufe von links nach rechts. Wenn bucket_store[r] - 1 eine L position ist, schreibe die Position an die erste freie Position in ihrem Bucket
    for r in 0..bucket_store.len() {
        if bucket_store.get(r) == invalid || bucket_store.get(r) == 0 {
            continue;
        }
        let pos = bucket_store.get(r) - 1;
        if ls.is_l(pos) {
            let c = s.get(pos as u64).as_();
            let c_pos = existing_chars.rank1(c) - 1;
            bucket_store.set(bucket_start.get(c_pos) as u64, pos);
            bucket_start.set(c_pos, bucket_start.get(c_pos) + 1);
        }
    }
    drop(bucket_start);

    //Schritt b: Durchlaufe von rechts nach links: Wenn bucket_store[r] - 1 eine S position ist, trage sie in ihrem Bucket ein
    for r in (0..bucket_store.len()).rev() {
        if bucket_store.get(r) == 0 {
            continue;
        }
        let pos = bucket_store.get(r) - 1;
        if ls.is_s(pos) {
            let c = s.get(pos as u64).as_();
            let c_pos = existing_chars.rank1(c) - 1;
            bucket_store.set(bucket_end.get(c_pos) as u64 - 1, pos);
            bucket_end.set(c_pos, bucket_end.get(c_pos) - 1);
        }
    }
    drop(bucket_end);
    drop(existing_chars);

    let mut res = IntVector::with_capacity(size_bits, lms_pos.len() as u64);
    bucket_store
        .into_iter()
        .filter(|&pos| ls.is_lms(pos))
        .for_each(|pos| res.push(pos));
    res
}

fn bucket_info(
    s: &IntVector<usize>,
    symbol_cnt: usize,
) -> (Rank9<BitVector<u64>>, IntVector<usize>, IntVector<usize>) {
    let s = s.into_iter();
    let n = s.len();
    let bits = (n as f64 + 1.0).log2().ceil() as usize;
    let mut bucket_sizes = IntVector::<usize>::with_fill(bits, symbol_cnt as u64, 0);
    let mut existing = BitVector::<u64>::with_fill(symbol_cnt as u64, false);
    for c in s {
        let index = c.as_();
        bucket_sizes.set(index, bucket_sizes.get(index) + 1);
        existing.set_bit(index as u64, true);
    }
    let existing = Rank9::new(existing);
    let existing_count = existing.rank1(existing.bit_len() - 1);
    let mut bucket_start = IntVector::with_capacity(bits, existing_count);
    let mut bucket_end = IntVector::with_capacity(bits, existing_count);

    bucket_sizes
        .into_iter()
        .filter(|&size| size > 0)
        .scan(0usize, |state, size| {
            let old_state = *state;
            *state += size;
            Some((old_state, *state))
        })
        .for_each(|(start, end)| {
            bucket_start.push(start);
            bucket_end.push(end);
        });
    (existing, bucket_start, bucket_end)
}
