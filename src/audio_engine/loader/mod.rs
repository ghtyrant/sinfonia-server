pub mod base;
mod sndfile;

use std::ffi::OsStr;
use std::path::PathBuf;

use audio_engine::loader::base::AudioFileLoader;
use audio_engine::loader::sndfile::SndFileLoader;

use error::SinfoniaGenericError;

pub fn get_loader_for_file(path: &PathBuf) -> Result<impl AudioFileLoader, SinfoniaGenericError> {
    let ext = path.extension().and_then(OsStr::to_str);
    match ext {
        Some("wav") | Some("ogg") => Ok(SndFileLoader {}),
        _ => {
            error!("No loader installed for extension {}", ext.unwrap());
            Err(SinfoniaGenericError::UnsupportedFileFormat(ext.unwrap().into(), path.to_string_lossy().into_owned()))
        }
    }
}
