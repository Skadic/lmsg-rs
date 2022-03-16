pub mod iss;
pub mod lmsg;

#[cfg(test)]
mod test {
    use crate::iss;
    use crate::lmsg;
    use std::time::Instant;

    #[test]
    fn run() {
        let mut input = std::fs::read("res/dna.10MB.txt").unwrap();
        let input2 = b"gccttaacattattacgccta";
        let input3 = b"gccttaacattattacgcctaagcfsadfsdfffsdfstaasdfcgacgtagctatcgtagctacgtactagt";

        let rules = lmsg::compress_raw(input);

        for (i, rule) in rules.into_iter().enumerate() {
            let rhs = rule.into_iter().map(|v| {
                if v < 256 {
                    format!("{} ", match v as u8 as char {
                        '\0' => "\\0".to_owned(),
                        x => x.to_string()
                    })
                } else {
                    format!("R{} ", v - 256)
                }
            }).collect::<String>();
            println!("R{i} -> {rhs}");
        }
    }
}
