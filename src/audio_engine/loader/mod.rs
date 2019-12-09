pub mod base;
pub mod error;
mod minimp3;
mod sndfile;

use std::ffi::OsStr;
use std::path::PathBuf;

use crate::audio_engine::loader::base::AudioFileLoader;
use crate::audio_engine::loader::minimp3::MiniMP3Loader;
use crate::audio_engine::loader::sndfile::SndFileLoader;

use crate::audio_engine::loader::error::AudioFileLoaderError;

pub fn get_loader_for_file(
    path: &PathBuf,
) -> Result<Box<dyn AudioFileLoader>, AudioFileLoaderError> {
    let ext = path.extension().and_then(OsStr::to_str);
    match ext {
        Some("mp3") => Ok(Box::new(MiniMP3Loader {})),
        Some("wav") | Some("ogg") => Ok(Box::new(SndFileLoader {})),

        _ => {
            error!("No loader installed for extension {}", ext.unwrap());
            Err(AudioFileLoaderError::UnsupportedFileFormat(
                ext.unwrap().into(),
                path.to_string_lossy().into_owned(),
            ))
        }
    }
}
