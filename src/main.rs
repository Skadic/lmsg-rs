pub mod iss;
pub mod lmsg;

use std::time::Instant;

fn main() {
    let input = std::fs::read("res/dna.50MB").unwrap();
    let input2 = b"gccttaacattattacgccta";
    let _input3 = b"gccttaacattattacgcctaagcfsadfsdfffsdfstaasdfcgacgtagctatcgtagctacgtactagt";

    let now = Instant::now();
    let rules = lmsg::compress_raw(input);
    //print_rules(rules.iter());
    println!("{}ms", now.elapsed().as_millis())
}

fn print_rules(rules: impl IntoIterator<Item = impl IntoIterator<Item = usize>>) {
    for (i, rule) in rules.into_iter().enumerate() {
        let rhs = rule
            .into_iter()
            .map(|v| {
                if v < 256 {
                    format!(
                        "{} ",
                        match v as u8 as char {
                            '\0' => "\\0".to_owned(),
                            x => x.to_string(),
                        }
                    )
                } else {
                    format!("R{} ", v - 256)
                }
            })
            .collect::<String>();
        println!("R{i} -> {rhs}");
    }
}
