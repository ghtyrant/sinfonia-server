use alto;
use alto::Source;

use std::path::PathBuf;
use std::sync::Arc;

use audio_engine::backends::base::{AudioBackend, AudioEntityData};
use audio_engine::loader::base::AudioFileLoader;
use audio_engine::loader;

pub struct OpenALAudioEntityData {
    buffer: Arc<alto::Buffer>,
    source: Option<alto::StaticSource>,
}

impl AudioEntityData for OpenALAudioEntityData {
    type AudioBackend = OpenALAudioBackend;

    fn pause(&mut self) {
        if let Some(ref mut src) = self.source {
            src.pause();
        }
    }

    fn play(&mut self) {
        if let Some(ref mut src) = self.source {
            src.set_buffer(self.buffer.clone()).unwrap();
            src.play();
        }
    }
}

pub struct OpenALAudioBackend {
    alto: alto::Alto,
    context: alto::Context,
}

impl AudioBackend for OpenALAudioBackend {
    type AudioBackendEntityData = OpenALAudioEntityData;

    fn init() -> Self {
        let alto = if let Ok(alto) = alto::Alto::load_default() {
            alto
        } else {
            panic!("No OpenAL implementation present!");
        };

        for s in alto.enumerate_outputs() {
            println!("Found device: {}", s.to_str().unwrap());
        }

        println!("Using output: {:?}", alto.default_output().unwrap());
        let dev = alto.open(None).unwrap();
        let ctx = dev
            .new_context(Some(alto::ContextAttrs {
                frequency: None,
                refresh: None,
                mono_sources: None,
                stereo_sources: None,
                soft_hrtf_id: None,
                soft_hrtf: None,
                soft_output_limiter: None,
                max_aux_sends: Some(8),
            })).unwrap();

        OpenALAudioBackend {
            alto: alto,
            context: ctx,
        }
    }

    fn load_object(&mut self, path: &PathBuf) -> Self::AudioBackendEntityData {
        let (mut samples, sample_rate) = loader::get_loader_for_file(path).unwrap().load(path);
        let converted_samples: Vec<alto::Mono<i16>> = samples
            .drain(0..)
            .map(|v| alto::Mono { center: v })
            .collect();

        let buf = self
            .context
            .new_buffer(converted_samples, sample_rate)
            .unwrap();
        let buf = Arc::new(buf);

        Self::AudioBackendEntityData {
            buffer: buf,
            source: None,
        }
    }

    fn set_volume(&mut self, volume: f32) {
        self.context.set_gain(volume);
    }

    fn get_output_devices(&mut self) -> Vec<String> {
        self.alto
            .enumerate_outputs()
            .into_iter()
            .map(|d| (d.to_str().unwrap().to_owned()))
            .collect()
    }

    fn get_current_output_device(&mut self) -> i32 {
        // TODO implement
        0
    }

    fn set_current_output_device(&mut self, _id: i32) {
        // TODO implement
    }

    fn play(&mut self, object: &mut Self::AudioBackendEntityData) {
        let src = self.context.new_static_source().unwrap();
        object.source = Some(src);
        object.play();
    }
}
