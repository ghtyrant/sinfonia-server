use std::path::PathBuf;

use error::SinfoniaGenericError;

pub trait AudioFileLoader {
  fn load(&mut self, path: &PathBuf) -> Result<(Vec<i16>, i32), SinfoniaGenericError>;
}
