//! The game files the Activity hands over.

use std::path::PathBuf;

/// The path of a `file://` URL, as the Activity sends for a game another app
/// opened with FastEmu. Other schemes are not files this app can read.
pub fn file_url_path(url: &str) -> Option<PathBuf> {
    let encoded = url.strip_prefix("file://")?;
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = encoded.get(i + 1..i + 3)?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(decoded)
        .ok()
        .filter(|path| path.starts_with('/'))
        .map(PathBuf::from)
}

/// The paths of the picker's answer: the copies of the picked files, one
/// per line.
pub fn picked_paths(answer: &str) -> Vec<PathBuf> {
    answer
        .lines()
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_urls_decode_to_paths() {
        assert_eq!(
            file_url_path("file:///data/user/0/io.github.fzn0x.fastemu/files/games/My%20Game.jar"),
            Some(PathBuf::from(
                "/data/user/0/io.github.fzn0x.fastemu/files/games/My Game.jar"
            ))
        );
        assert_eq!(
            file_url_path("file:///sdcard/%E3%82%B2%E3%83%BC%E3%83%A0.jad"),
            Some(PathBuf::from("/sdcard/\u{30b2}\u{30fc}\u{30e0}.jad"))
        );
        assert_eq!(file_url_path("content://media/1"), None);
        assert_eq!(file_url_path("file://relative.jar"), None);
        assert_eq!(file_url_path("file:///bad%2"), None);
        assert_eq!(file_url_path("file:///bad%zz"), None);
    }

    #[test]
    fn picked_paths_come_one_per_line() {
        assert_eq!(
            picked_paths("/g/a/Game.jad\n/g/a/Game.jar\n"),
            [
                PathBuf::from("/g/a/Game.jad"),
                PathBuf::from("/g/a/Game.jar")
            ]
        );
        assert!(picked_paths("").is_empty());
    }
}
