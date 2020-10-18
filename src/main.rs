use audio_db_tags::Id3v2;

fn main() {
    let id3v2 = Id3v2::from_path(
        "/home/sam/Music/KOAN Sound/Intervals Above/KOAN Sound - Intervals Above - 01 Strident.mp3",
    );

    match id3v2 {
        Ok(id3v2) => println!("{:#?}", id3v2),
        Err(e) => println!("Error: {}", e),
    }
}
