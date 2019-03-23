use std::path::PathBuf;

use error::SinfoniaGenericError;

pub trait AudioEntityData: Sized {
    type Backend: AudioBackend;

    fn pause(&mut self);
    fn play(&mut self, backend: &mut Self::Backend);
    fn stop(&mut self, backend: &mut Self::Backend);
    fn is_playing(&mut self) -> bool;
    fn set_volume(&mut self, volume: f32);
    fn set_pitch(&mut self, pitch: f32);
    fn set_lowpass(&mut self, amount: f32);
    fn set_reverb(&mut self, reverb: &str);
    fn get_position(&mut self) -> f32;
}

pub trait AudioBackend: Sized {
    type EntityData: AudioEntityData<Backend = Self>;

    fn init() -> Self;
    fn load_file(&mut self, path: &PathBuf) -> Result<Self::EntityData, SinfoniaGenericError>;
    fn set_volume(&mut self, volume: f32);
    fn get_output_devices(&mut self) -> Vec<String>;

    fn get_current_output_device(&mut self) -> i32;
    fn set_current_output_device(&mut self, id: i32);
}
