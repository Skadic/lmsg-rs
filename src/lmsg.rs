use std::{collections::HashMap};

use crate::iss;


use succinct::{
    BitRankSupport, BitVec, BitVecMut, BitVecPush, BitVector, IntVec, IntVector, Rank9,
};

pub fn compress(s: impl AsRef<str>) -> Vec<Vec<usize>> {
    compress_multiple(std::iter::once(s))
}

pub fn compress_raw(it: impl IntoIterator<Item = u8>) -> Vec<Vec<usize>> {
    compress_multiple_raw(std::iter::once(it))
}

pub fn compress_multiple(iterable: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<Vec<usize>> {
    compress_multiple_raw(
        iterable
            .into_iter()
            .map(|v| v.as_ref().bytes().collect::<Vec<_>>()),
    )
}

// TODO Grammatik bereinigen
pub fn compress_multiple_raw(iterable: impl IntoIterator<Item = impl IntoIterator<Item = u8>>) -> Vec<Vec<usize>> {
    let mut input = iterable
        .into_iter()
        .flat_map(|v| v.into_iter().chain(std::iter::once(0)))
        .map(|v| v as usize)
        .collect::<Vec<usize>>();
    let mut rules: Vec<Vec<usize>> = vec![];

    let mut ls: iss::LS;
    let mut lms_pos: IntVector<usize>;

    loop {
        let rules_before = rules.len();
        let mut max_symbol = 256 + rules_before;

        // Create DS
        ls = iss::LS::new(&input);
        lms_pos = iss::iss_with_ls(&input, &ls, max_symbol);

        // The end indices (exclusive) of the lms substrings
        let lms_substring_ends = lms_pos
            .iter()
            .map(|v| ls.next_lms_index(v).unwrap_or(input.len() - 1) + 1)
            .collect::<Vec<usize>>();
        // Build a BitVector that is 1 if the ith and i-1th lms substrings are the same
        let mut same_lms_str = BitVector::<u64>::with_capacity(lms_pos.len());
        same_lms_str.push_bit(false);
        for i in 0..lms_pos.len() as usize - 1 {
            let substr1 = &input[lms_pos.get(i as u64)..lms_substring_ends[i]];
            let substr2 = &input[lms_pos.get(i as u64 + 1)..lms_substring_ends[i + 1]];
            same_lms_str.push_bit(substr1 == substr2);
        }
        let mut done = BitVector::<usize>::with_fill(lms_pos.len(), false);

        for i in 0..lms_pos.len() as usize {
            if done.get_bit(i as u64) {
                continue;
            }
            let lms = lms_pos.get(i as u64);

            // Since lms substrings overlap at their start and end index, we skip each first char
            let lms_start = lms;
            let lms_end = lms_substring_ends[i] - 1;

            let mut r = i + 1;
            while r < lms_pos.len() as usize - 1 && same_lms_str.get_bit(r as u64) {
                r += 1;
            }
            // The commented out stuff is only possible if r - i is less than the amount of bits usize is wide. sadge
            //let bits = !(usize::MAX << (r - i));
            //done.set_bits(i as u64, r - i, bits);
            for k in i..r {
                done.set_bit(k as u64, true)
            }

            if lms_start < lms_end {
                rules.push((&input[lms_start..lms_end]).iter().copied().collect());
                max_symbol += 1;
            }
        }
        // in this case, no new rules have been created. We are done
        if rules_before == rules.len() {
            break;
        }

        // A map that allows getting the index in lms_pos for every index that is an lms_pos
        let mut lms_index_map = HashMap::<usize, usize>::new();
        for (i, lms) in lms_pos.iter().enumerate() {
            lms_index_map.insert(lms, i);
        }
        
        // TODO unötiges Zeug droppen

        // Build a rank DS over the bv
        // This allows us to get the index of the corresponding rule for every lms substring.
        let same_lms_str_rank = Rank9::new(same_lms_str);

        let mut new_input = Vec::with_capacity(input.len() / 2 + 1);
        let mut i = 0;
        while i < input.len() - 1 {
            if ls.is_lms(i) {
                // - 1 because rank "starts counting" at 1 instead of 0, and another -1 because we skip inserting the rule for the sentinel, since it's empty anyway
                let rule_id = same_lms_str_rank.rank0(lms_index_map[&i] as u64) as usize - 1 - 1;
                new_input.push(rule_id + rules_before + 256);
                i += std::cmp::max(rules[rule_id].len(), 1);
            } else {
                new_input.push(input[i]);
                i += 1;
            }
        }
        // the sentinel
        new_input.push(0);
        input = new_input;
    }
    rules.push(input);

    rules
}
