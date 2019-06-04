#![recursion_limit="256"]
#[macro_use]
extern crate quote;

extern crate syn;

extern crate proc_macro2;

extern crate crc16;
extern crate xml;

mod parser;
mod mavmessage;

use std::env;
use std::fs::File;
use std::path::Path;

pub fn main() {
    let src_dir = env::current_dir().unwrap();
    let in_path = Path::new(&src_dir).join("common.xml");
    let mut inf = File::open(&in_path).unwrap();
    
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);
    //let mut outf = File::create(&dest_path).unwrap();

    parser::generate(&mut inf, &out_path);

    // TODO: remove once stable
    // Re-run build only if common.xml changes
    // see: https://doc.rust-lang.org/cargo/reference/build-scripts.html
    // for details.
    println!("cargo:rerun-if-changed=common.xml");

}
