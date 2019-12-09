use crate::audio_engine::loader::error::AudioFileLoaderError;
use failure::Fail;

#[derive(Fail, Debug)]
pub enum AudioBackendError {
  #[fail(display = "AudioBackend AltoError: {}", _0)]
  AltoError(alto::AltoError),

  #[fail(display = "AudioBackend Operation on empty source!")]
  NoSource,

  #[fail(display = "AudioBackend FileLoader Error: {}", _0)]
  AudioFileLoaderError(AudioFileLoaderError),
}

impl From<alto::AltoError> for AudioBackendError {
  fn from(e: alto::AltoError) -> Self {
    Self::AltoError(e)
  }
}

impl From<AudioFileLoaderError> for AudioBackendError {
  fn from(e: AudioFileLoaderError) -> Self {
    Self::AudioFileLoaderError(e)
  }
}
