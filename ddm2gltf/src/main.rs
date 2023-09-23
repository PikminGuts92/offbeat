use offbeat::*;
use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();

    let ddm_file_path = Path::new(&args[0]);
    let mut ddm_file = File::open(ddm_file_path).unwrap();
    let ddm = DdmFile::from_file(&mut ddm_file).unwrap();

    println!("{ddm:#?}");
}
