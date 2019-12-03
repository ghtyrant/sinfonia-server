use failure::Fail;

#[derive(Fail, Debug)]
pub enum AudioFileLoaderError {
  #[fail(display = "Failed to load file '{}': {}", _0, _1)]
  FileLoadError(String, String),

  #[fail(display = "Unsupported file format '{}' for file '{}'", _0, _1)]
  UnsupportedFileFormat(String, String),
}
