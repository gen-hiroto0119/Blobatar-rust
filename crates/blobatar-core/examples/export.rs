use blobatar_core::{Avatar, Options};

fn main() {
    let options = Options::default();
    let avatar = Avatar::new("blobatar", &options);
    println!("{}", avatar.svg(&options));
}
