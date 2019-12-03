use failure::Fail;

#[derive(Fail, Debug)]
pub enum SamplesDBError {
  #[fail(display = "SamplesDB Sqlite Error: {}", _0)]
  SqliteError(rusqlite::Error),

  #[fail(display = "SamplesDB WalkDir Error: {}", _0)]
  WalkDirError(walkdir::Error),

  #[fail(display = "SamplesDB Failed to create tag '{}'", _0)]
  TagCreationError(String),
}

impl From<rusqlite::Error> for SamplesDBError {
  fn from(e: rusqlite::Error) -> Self {
    Self::SqliteError(e)
  }
}

impl From<walkdir::Error> for SamplesDBError {
  fn from(e: walkdir::Error) -> Self {
    Self::WalkDirError(e)
  }
}
