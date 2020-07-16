use multihash::Blake3;

fn main() {
    let hash = Blake3::digest(b"hello");
    println!("{}", hex::encode(hash));
}