use std::path::PathBuf;

pub trait AudioFileLoader {
    fn load(&mut self, path: &PathBuf) -> (Vec<i16>, i32);
}