use std::{fs::File, time::SystemTime};

fn main() {
    println!("cargo:warning=Debug: build script is running!");

    // Touch the assets file so `include_dir!()`` will update.
    let assets_file = File::open("src/assets.rs").unwrap();
    assets_file.set_modified(SystemTime::now()).unwrap();
}
