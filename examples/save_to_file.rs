use std::fs::File;
use std::io::BufWriter;

use bincode;

use spyglass::SuffixTrie;

fn main() {
    let directory = std::env::args().nth(1).expect("No input directory given");
    let output_file = std::env::args().nth(2).expect("No output file specified");
    let trie = SuffixTrie::from_directory(&directory).unwrap();

    let mut f = BufWriter::new(File::create(&output_file).unwrap());
    bincode::serialize_into(&mut f, &trie).unwrap();
}
