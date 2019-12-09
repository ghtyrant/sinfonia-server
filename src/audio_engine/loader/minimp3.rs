use std::error::Error;
use std::fs::File;
use std::path::PathBuf;

use minimp3::{Decoder, Error as MiniMP3Error, Frame};

use crate::audio_engine::loader::base::AudioFileLoader;
use crate::audio_engine::loader::error::AudioFileLoaderError;
use crate::utils::convert_to_mono;

pub struct MiniMP3Loader;

impl AudioFileLoader for MiniMP3Loader {
    fn load(&mut self, path: &PathBuf) -> Result<(Vec<i16>, i32), AudioFileLoaderError> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                return Err(AudioFileLoaderError::FileLoadError(
                    path.to_string_lossy().into_owned(),
                    e.description().to_string(),
                ))
            }
        };

        let mut decoder = Decoder::new(file);

        let mut samples = Vec::new();
        let mut final_sample_rate = 0;
        loop {
            match decoder.next_frame() {
                Ok(Frame {
                    mut data,
                    sample_rate,
                    channels,
                    ..
                }) => {
                    final_sample_rate = sample_rate;
                    if channels == 2 {
                        samples.append(&mut convert_to_mono(data));
                    } else {
                        samples.append(&mut data);
                    }
                }
                Err(MiniMP3Error::Eof) => break,
                Err(e) => {
                    return Err(AudioFileLoaderError::FileLoadError(
                        path.to_string_lossy().into_owned(),
                        e.description().to_string(),
                    ));
                }
            }
        }

        Ok((samples, final_sample_rate))
    }
}
