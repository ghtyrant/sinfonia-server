pub mod base;
mod minimp3;
mod sndfile;

use std::ffi::OsStr;
use std::path::PathBuf;

use audio_engine::loader::base::AudioFileLoader;
use audio_engine::loader::minimp3::MiniMP3Loader;
use audio_engine::loader::sndfile::SndFileLoader;

use error::SinfoniaGenericError;

pub fn get_loader_for_file(path: &PathBuf) -> Result<Box<AudioFileLoader>, SinfoniaGenericError> {
    let ext = path.extension().and_then(OsStr::to_str);
    match ext {
        Some("mp3") => Ok(Box::new(MiniMP3Loader {})),
        Some("wav") | Some("ogg") => Ok(Box::new(SndFileLoader {})),

        _ => {
            error!("No loader installed for extension {}", ext.unwrap());
            Err(SinfoniaGenericError::UnsupportedFileFormat(
                ext.unwrap().into(),
                path.to_string_lossy().into_owned(),
            ))
        }
    }
}
