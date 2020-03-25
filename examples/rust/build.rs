use std::env;

fn main()
{
    let project_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    //println!("cargo:rustc-link-search={}/../bin/", project_dir); // the "-L" flag
    //println!("cargo:rustc-link-lib=mothra"); // the "-l" flag
}