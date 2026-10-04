use blobatar_core::{Avatar, Options};

fn main() {
    let options = Options::default();
    let seed = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "blobatar".to_string());
    let avatar = Avatar::new(&seed, &options);
    println!("{}", avatar.svg(&options));
}
