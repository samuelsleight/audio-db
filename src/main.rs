use audio_db_tags::Flac;

fn main() {
    let flac = Flac::from_path("/home/sam/Music/KOAN Sound/Intervals Above/KOAN Sound - Intervals Above - 01 Strident.flac");
    println!("{:?}", flac);
}
