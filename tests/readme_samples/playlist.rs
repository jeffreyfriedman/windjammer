struct Playlist {
    name: String,
    songs: Vec<String>,
}

fn add_song(list: &mut Playlist, song: &str) {
    list.songs.push(song.to_string());
}

fn count(list: &Playlist) -> usize {
    list.songs.len()
}

fn describe(list: &Playlist) -> String {
    format!("{}: {}", list.name, list.songs.join(", "))
}

fn main() {
    let mut list = Playlist { name: "Road trip".to_string(), songs: vec![] };
    add_song(&mut list, "Africa");
    add_song(&mut list, "Hey Jude");
    println!("{}", describe(&list));
    println!("{} songs", count(&list));
}
