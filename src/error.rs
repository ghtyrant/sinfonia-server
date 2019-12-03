use audio_engine::backends::error::AudioBackendError;
use samplesdb::error::SamplesDBError;
use std::convert::From;

use failure::Fail;

#[derive(Fail, Debug)]
pub enum SinfoniaGenericError {
  #[fail(display = "SamplesDB Sqlite Error: {}", _0)]
  SamplesDBError(SamplesDBError),

  #[fail(display = "AudioBackendError: {}", _0)]
  AudioBackendError(AudioBackendError),
}

impl From<SamplesDBError> for SinfoniaGenericError {
  fn from(e: SamplesDBError) -> Self {
    Self::SamplesDBError(e)
  }
}

impl From<AudioBackendError> for SinfoniaGenericError {
  fn from(e: AudioBackendError) -> Self {
    Self::AudioBackendError(e)
  }
}
