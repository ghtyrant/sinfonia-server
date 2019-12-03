use std::path::PathBuf;

use crate::audio_engine::loader::error::AudioFileLoaderError;

pub trait AudioFileLoader {
  fn load(&mut self, path: &PathBuf) -> Result<(Vec<i16>, i32), AudioFileLoaderError>;
}
