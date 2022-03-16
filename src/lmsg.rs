use std::{cmp::Ordering, collections::HashMap};

use crate::iss::{InducedSuffixSort, LS};

use num::{cast::AsPrimitive, Integer, Unsigned};
use succinct::{
    storage::BlockType, BitRankSupport, BitVec, BitVecMut, BitVecPush, BitVector, IntVec,
    IntVecMut, IntVector, Rank9,
};

pub fn compress(s: String) -> Vec<IntVector<usize>> {
    compress_raw(s.into_bytes())
}

pub fn compress_multiple(iterable: impl IntoIterator<Item = String>) -> Vec<IntVector<usize>> {
    compress_multiple_raw(iterable.into_iter().map(|v| v.into_bytes()))
}

// TODO Grammatik bereinigen
pub fn compress_multiple_raw(
    iterable: impl IntoIterator<Item = impl IntoIterator<Item = u8>>,
) -> Vec<IntVector<usize>> {
    let mut input = iterable
        .into_iter()
        .flat_map(|v| v.into_iter().chain(std::iter::once(0)))
        .collect::<Vec<u8>>();
    // Remove the last sentinel, since compress_raw will add one
    input.pop();
    compress_raw(input)
}

fn cmp_vec_range<B>(
    v1: &impl IntVec<Block = B>,
    v2: &impl IntVec<Block = B>,
    i1: u64,
    j1: u64,
    i2: u64,
    j2: u64,
) -> Ordering
where
    B: BlockType,
{
    use std::cmp::Ordering::*;
    for i in 0..u64::min(j1 - i1, j2 - i2) {
        let ordering = v1.get(i1 + i).cmp(&v2.get(i2 + i));
        if ordering != Equal {
            return ordering;
        }
    }
    u64::cmp(&(j1 - i1), &(j2 - i2))
}

pub fn compress_raw(data: Vec<u8>) -> Vec<IntVector<usize>> {
    let mut input = IntVector::<usize>::with_capacity(8, data.len() as u64 + 1);
    data.iter().for_each(|&v| input.push(v as usize));
    input.push(0);
    drop(data);
    let mut rules: Vec<IntVector<usize>> = vec![];

    loop {
        let rules_before = rules.len();
        let max_symbol = 256 + rules_before;
        let input_bits = (input.len() as f64 + 1.0).log2().ceil() as usize;
        let invalid = !(usize::MAX << input_bits);
        // Create DS
        let ls = LS::from(&input);
        let lms_pos = input.iss_with_ls(&ls, max_symbol); //crate::iss::iss::<IntVector<usize>, usize>(&input, max_symbol);

        // The end indices (exclusive) of the lms substrings
        let mut lms_substring_ends = IntVector::with_capacity(input_bits, lms_pos.len());
        lms_pos
            .iter()
            .map(|v| ls.next_lms_index(v).unwrap_or(input.len() as usize - 1) + 1)
            .for_each(|end| lms_substring_ends.push(end));
        // Build a BitVector that is 1 if the ith and i-1th lms substrings are the same
        let mut same_lms_str = BitVector::<u64>::with_capacity(lms_pos.len());
        same_lms_str.push_bit(false);
        for i in 0..lms_pos.len() - 1 {
            same_lms_str.push_bit(
                cmp_vec_range(
                    &input,
                    &input,
                    lms_pos.get(i) as u64,
                    lms_substring_ends.get(i) as u64,
                    lms_pos.get(i + 1) as u64,
                    lms_substring_ends.get(i + 1) as u64,
                ) == Ordering::Equal,
            );
        }

        let additional_rules = generate_rules(
            &lms_pos,
            lms_substring_ends.iter(),
            &same_lms_str,
            &mut rules,
            &input,
            max_symbol,
        );
        drop(lms_substring_ends);
        drop(ls);
        // We have new rules, therefore new characters and might need more bits to represent them
        let symbol_bits = ((max_symbol + additional_rules) as f64).log2().ceil() as usize;
        // in this case, no new rules have been created. We are done
        if additional_rules == 0 {
            break;
        }

        // In this case, we don't have enough bits to represent all new characters
        // We need to reallocate input
        if symbol_bits > input.element_bits() {
            let mut new_input = IntVector::with_capacity(symbol_bits, input.len());
            input.into_iter().for_each(|v| new_input.push(v));
            input = new_input;
        }

        // A map that allows getting the index in lms_pos for every index that is an lms_pos
        let mut lms_index_map = IntVector::<usize>::with_fill(input_bits, input.len() + 1, invalid);

        for (i, lms) in lms_pos.iter().enumerate() {
            lms_index_map.set(lms as u64, i);
        }
        drop(lms_pos);

        // Build a rank DS over the bv
        // This allows us to get the index of the corresponding rule for every lms substring.
        let same_lms_str_rank = Rank9::new(same_lms_str);

        let mut i = 0;
        let mut new_len = 0;
        while i < input.len() - 1 {
            if lms_index_map.get(i) as usize != invalid {
                // - 1 because rank "starts counting" at 1 instead of 0, and another -1 because we skip inserting the rule for the sentinel, since it's empty anyway
                let rule_id =
                    same_lms_str_rank.rank0(lms_index_map.get(i as u64) as u64) as usize - 1 - 1;
                input.set(new_len, rule_id + rules_before + 256);
                i += std::cmp::max(rules[rule_id].len(), 1);
            } else {
                input.set(new_len, input.get(i as u64));
                i += 1;
            }
            new_len += 1;
        }
        // the sentinel
        input.set(new_len, 0);
        new_len += 1;
        input.truncate(new_len);
    }
    rules.push(input);

    rules
}

fn generate_rules<LmsInt, LmsIter, EndInt, EndIter, BlockT>(
    lms_pos: LmsIter,
    lms_substring_ends: EndIter,
    same_lms_str: &impl BitVec,
    rules: &mut Vec<IntVector<BlockT>>,
    input: &impl IntVec<Block = BlockT>,
    max_symbol: usize,
) -> usize
where
    LmsInt: Unsigned + Integer + BlockType + Copy + AsPrimitive<usize>,
    LmsIter: IntoIterator<Item = LmsInt>,
    LmsIter::IntoIter: ExactSizeIterator,
    EndInt: Unsigned + Integer + BlockType + Copy + AsPrimitive<usize>,
    EndIter: IntoIterator<Item = EndInt>,
    EndIter::IntoIter: ExactSizeIterator,
    BlockT: BlockType,
{
    let lms_pos = lms_pos.into_iter();
    let n = lms_pos.len();
    let mut done = BitVector::<usize>::with_fill(n as u64, false);
    let symbol_bits = (max_symbol as f64).log2().ceil() as usize;
    let mut new_symbol_count = 0;

    for (i, (lms, lms_end)) in lms_pos.zip(lms_substring_ends.into_iter()).enumerate() {
        if done.get_bit(i as u64) {
            continue;
        }

        // Since lms substrings overlap at their start and end index, we skip each first char
        let lms_substring_start = lms.as_();
        let lms_substring_end = lms_end.as_() - 1;

        let mut r = i + 1;
        while r < n - 1 && same_lms_str.get_bit(r as u64) {
            r += 1;
        }
        // The commented out stuff is only possible if r - i is less than the amount of bits usize is wide. sadge
        //let bits = !(usize::MAX << (r - i));
        //done.set_bits(i as u64, r - i, bits);
        for k in i..r {
            done.set_bit(k as u64, true)
        }

        if lms_substring_start < lms_substring_end {
            let mut vec = IntVector::<BlockT>::with_capacity(
                symbol_bits,
                (lms_substring_end - lms_substring_start) as u64,
            );
            for i in lms_substring_start..lms_substring_end {
                vec.push(input.get(i as u64));
            }
            rules.push(vec);
            new_symbol_count += 1;
        }
    }
    new_symbol_count
}
