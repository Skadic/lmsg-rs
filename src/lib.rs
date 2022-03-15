pub mod iss;

#[cfg(test)]
mod test {
    use crate::iss;
    use std::time::Instant;

    #[test]
    fn run() {
        let mut input = std::fs::read("res/dna.1MB.txt").unwrap();
        input.push(0);
        let mut now = Instant::now();
        let a = iss::iss(&input);
        println!("{}ms", now.elapsed().as_millis());
        panic!();
    }
}
