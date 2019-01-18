use audio_engine::messages::command::LoadTheme;
use std::sync::mpsc::SendError;

use failure::{Fail};

#[derive(Fail, Debug)]
pub enum SinfoniaGenericError {
    #[fail(display = "GenericError")]
    GenericError,

    #[fail(display = "Failed to parse JSON: {}", _0)]
    JSONParseError(String),

    #[fail(display = "Failed to load file '{}': {}", _0, _1)]
    FileLoadError(String, String),

    #[fail(display = "Unsupported file format '{}' for file '{}'", _0, _1)]
    UnsupportedFileFormat(String, String),
}