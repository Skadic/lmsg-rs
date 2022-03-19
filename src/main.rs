pub mod iss;
pub mod lmsg;

use std::time::Instant;

fn main() {
    let input = std::fs::read("res/dna.50MB_prefix_1MB").unwrap();
    let _input2 = "gccttaacattattacgccta".to_owned();
    let _input3 =
        "gccttaacattattacgcctaagcfsadfsdfffsdfstaasdfcgacgtagctatcgtagctacgtactagt".to_owned();

    let now = Instant::now();
    let _rules = lmsg::compress(_input2);
    print_rules(_rules.iter());
    println!("{}ms", now.elapsed().as_millis())
}

#[allow(dead_code)]
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

// gccttaacattattacgccta
