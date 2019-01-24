use alto;
use alto::Source;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use audio_engine::backends::base::{AudioBackend, AudioEntityData};
use audio_engine::loader;
use audio_engine::loader::base::AudioFileLoader;
use error::SinfoniaGenericError;

pub struct OpenALEntityData {
    buffer: Arc<alto::Buffer>,
    source: Option<OpenALSource>,
}

impl AudioEntityData for OpenALEntityData {
    type Backend = OpenALBackend;

    fn pause(&mut self) {
        if let Some(ref mut src) = self.source {
            src.handle.pause();
        }
    }

    fn stop(&mut self, backend: &mut Self::Backend) {
        if let Some(ref mut src) = self.source {
            src.handle.stop();
        }

        backend.free_source(self.source.take().unwrap());
    }

    fn play(&mut self, backend: &mut Self::Backend) {
        self.source = backend.get_source();
        if let Some(ref mut src) = self.source {
            src.handle.set_buffer(self.buffer.clone()).unwrap();
            src.handle.play();
        }
    }

    fn is_playing(&mut self) -> bool {
        if let Some(ref mut src) = self.source {
            if src.handle.state() == alto::SourceState::Playing {
                return true;
            }
        }

        false
    }
}

pub struct OpenALSource {
    id: u32,
    used: bool,
    handle: alto::StaticSource,
}

pub struct OpenALBackend {
    alto: alto::Alto,
    context: alto::Context,
    sources: HashMap<u32, OpenALSource>,
}

impl OpenALBackend {
    fn get_source(&mut self) -> Option<OpenALSource> {
        let mut free_source = 0;
        for (id, source) in &self.sources {
            if !source.used {
                free_source = *id;
                break;
            }
        }

        if free_source > 0 {
            return self.sources.remove(&free_source);
        }

        return None;
    }

    fn free_source(&mut self, source: OpenALSource) {
        self.sources.insert(source.id, source);
    }
}

impl AudioBackend for OpenALBackend {
    type EntityData = OpenALEntityData;

    fn init() -> Self {
        let alto = if let Ok(alto) = alto::Alto::load_default() {
            alto
        } else {
            panic!("No OpenAL implementation present!");
        };

        for s in alto.enumerate_outputs() {
            info!("Found device: {}", s.to_string_lossy());
        }

        info!("Using output: {:?}", alto.default_output().unwrap());
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

        // Try to create a pool of 32 static sources
        let mut sources: HashMap<u32, OpenALSource> = HashMap::new();
        let mut num_sources = 0;
        for i in 0..32 {
            let src = match ctx.new_static_source() {
                Ok(source) => source,
                Err(_) => {
                    warn!("Failed to create 32 static sources, created {}", i);
                    break;
                }
            };

            sources.insert(
                i,
                OpenALSource {
                    id: i + 1,
                    used: false,
                    handle: src,
                },
            );
            num_sources += 1;
        }

        if num_sources == 0 {
            panic!("Failed to create a single static source, aborting ...");
        }

        OpenALBackend {
            alto: alto,
            context: ctx,
            sources: sources,
        }
    }

    fn load_file(&mut self, path: &PathBuf) -> Result<Self::EntityData, SinfoniaGenericError> {
        let (mut samples, sample_rate) = loader::get_loader_for_file(path)?.load(path)?;

        let converted_samples: Vec<alto::Mono<i16>> = samples
            .drain(0..)
            .map(|v| alto::Mono { center: v })
            .collect();

        let buf = self
            .context
            .new_buffer(converted_samples, sample_rate)
            .unwrap();
        let buf = Arc::new(buf);

        Ok(Self::EntityData {
            buffer: buf,
            source: None,
        })
    }

    fn set_volume(&mut self, volume: f32) {
        // TODO handle errors
        self.context.set_gain(volume).unwrap();
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
}
