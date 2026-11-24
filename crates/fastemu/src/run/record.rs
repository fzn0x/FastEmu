//! Frames as PNG files: the last one (`--screenshot`) and every Nth
//! presented one (`--record`).

use std::path::{Path, PathBuf};

use fastemu_core::Frame;

/// Writes `frame` as a PNG.
pub fn save_png(frame: &Frame, path: &Path) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut encoder =
        png::Encoder::new(std::io::BufWriter::new(file), frame.width(), frame.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer
        .write_image_data(&frame.to_rgba8())
        .map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())
}

/// Whether `name` is one of the files a [`Recorder`] saves.
fn is_frame_name(name: &str) -> bool {
    name.strip_prefix("frame-")
        .and_then(|rest| rest.strip_suffix(".png"))
        .is_some_and(|number| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
}

/// Saves every Nth presented frame into a folder as `frame-000001.png`,
/// `frame-000002.png` and so on, numbered in the order they were saved so
/// video tools can read them as a sequence.
#[derive(Debug)]
pub struct Recorder {
    dir: PathBuf,
    every: u64,
    /// The serial the next saved frame needs at least.
    due: u64,
    saved: u64,
}

impl Recorder {
    /// Records into `dir`, which is made if missing; `every` is at least 1.
    /// A folder that already holds recorded frames is refused: a shorter
    /// run would leave the older run's later frames at the end of the
    /// sequence.
    pub fn new(dir: PathBuf, every: u64) -> Result<Self, String> {
        let failed = |e: std::io::Error| format!("{}: {e}", dir.display());
        std::fs::create_dir_all(&dir).map_err(failed)?;
        for entry in std::fs::read_dir(&dir).map_err(failed)? {
            if is_frame_name(&entry.map_err(failed)?.file_name().to_string_lossy()) {
                return Err(format!(
                    "{} already holds recorded frames; record into an empty folder",
                    dir.display()
                ));
            }
        }
        let every = every.max(1);
        Ok(Self {
            dir,
            every,
            due: every,
            saved: 0,
        })
    }

    /// Takes a newly presented frame and saves it when it is due: frames
    /// N, 2N, 3N and on by serial. When the runner missed the due frame
    /// itself, the next one after it stands in.
    pub fn offer(&mut self, frame: &Frame) -> Result<(), String> {
        if frame.serial() < self.due {
            return Ok(());
        }
        let path = self.dir.join(format!("frame-{:06}.png", self.saved + 1));
        save_png(frame, &path).map_err(|e| format!("{}: {e}", path.display()))?;
        self.saved += 1;
        self.due = (frame.serial() / self.every + 1) * self.every;
        Ok(())
    }

    /// How many frames were saved.
    pub fn saved(&self) -> u64 {
        self.saved
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn every_nth_frame_is_saved_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("frames");
        let mut recorder = Recorder::new(out.clone(), 3).unwrap();
        // The runner saw serials 1 to 7, then missed 8 and 9.
        for serial in [1, 2, 3, 4, 5, 6, 7, 10, 11] {
            recorder
                .offer(&Frame::filled(4, 2, 0xFF00_00FF, serial))
                .unwrap();
        }
        assert_eq!(recorder.saved(), 3);
        assert_eq!(
            saved_names(&out),
            ["frame-000001.png", "frame-000002.png", "frame-000003.png"]
        );
        let decoder = png::Decoder::new(std::io::BufReader::new(
            std::fs::File::open(out.join("frame-000001.png")).unwrap(),
        ));
        let info = decoder.read_info().unwrap().info().clone();
        assert_eq!((info.width, info.height), (4, 2));
    }

    #[test]
    fn every_frame_with_one() {
        let dir = tempfile::tempdir().unwrap();
        let mut recorder = Recorder::new(dir.path().to_path_buf(), 1).unwrap();
        for serial in 1..=4 {
            recorder
                .offer(&Frame::filled(1, 1, 0xFF00_0000, serial))
                .unwrap();
        }
        assert_eq!(recorder.saved(), 4);
    }

    #[test]
    fn a_folder_with_frames_of_an_earlier_run_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut first = Recorder::new(dir.path().to_path_buf(), 1).unwrap();
        for serial in 1..=3 {
            first
                .offer(&Frame::filled(1, 1, 0xFF00_0000, serial))
                .unwrap();
        }
        let err = Recorder::new(dir.path().to_path_buf(), 1).unwrap_err();
        assert!(err.contains("already holds recorded frames"), "{err}");

        let other = tempfile::tempdir().unwrap();
        std::fs::write(other.path().join("notes.txt"), "kept").unwrap();
        std::fs::write(other.path().join("frame-cover.png"), "kept").unwrap();
        assert!(Recorder::new(other.path().to_path_buf(), 1).is_ok());
    }
}
