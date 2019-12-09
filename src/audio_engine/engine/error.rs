use crate::audio_engine::backends::error::AudioBackendError;
use failure::Fail;

#[derive(Fail, Debug)]
pub enum AudioEngineError {
  #[fail(display = "AudioEngine AudioBackend Error: {}", _0)]
  AudioBackendError(AudioBackendError),

  #[fail(display = "AudioEngine Sample not found at path {}", _0)]
  SampleNotFound(String),
}

impl From<AudioBackendError> for AudioEngineError {
  fn from(e: AudioBackendError) -> Self {
    Self::AudioBackendError(e)
  }
}
