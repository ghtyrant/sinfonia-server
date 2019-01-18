use std::path::PathBuf;

use error::SinfoniaGenericError;

pub trait AudioEntityData {
    type AudioBackend: AudioBackend;

    fn pause(&mut self);
    fn play(&mut self);
}

pub trait AudioBackend {
    type AudioBackendEntityData: AudioEntityData;

    fn init() -> Self;
    fn load_file(
        &mut self,
        path: &PathBuf,
    ) -> Result<Self::AudioBackendEntityData, SinfoniaGenericError>;
    fn set_volume(&mut self, volume: f32);
    fn get_output_devices(&mut self) -> Vec<String>;

    fn get_current_output_device(&mut self) -> i32;
    fn set_current_output_device(&mut self, id: i32);
    fn play(&mut self, object: &mut Self::AudioBackendEntityData);
}
