use alto;
use alto::efx::Filter;
use alto::Source;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use audio_engine::backends::base::{AudioBackend, AudioEntityData};
use audio_engine::loader;
use audio_engine::loader::base::AudioFileLoader;
use error::SinfoniaGenericError;

fn reverb_name_to_ref(reverb: &str) -> Option<&'static alto::efx::EaxReverbProperties> {
    match reverb {
        "none" => None,
        "underwater" => Some(&alto::efx::REVERB_PRESET_UNDERWATER),
        "forest" => Some(&alto::efx::REVERB_PRESET_FOREST),
        "spacestation" => Some(&alto::efx::REVERB_PRESET_SPACESTATION_LONGPASSAGE),
        "chapel" => Some(&alto::efx::REVERB_PRESET_CHAPEL),
        &_ => {
            warn!("Unknown reverb preset '{}'!", reverb);
            None
        }
    }
}

pub struct OpenALEntityData {
    buffer: Arc<alto::Buffer>,
    source: Option<OpenALSource>,
    lowpass: Option<alto::efx::LowpassFilter>,
    efx_slot: Option<alto::efx::AuxEffectSlot>,
    reverb: Option<alto::efx::ReverbEffect>,
    length: f32
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

        self.efx_slot = None;
        self.reverb = None;

        if self.source.is_some() {
            backend.free_source(self.source.take().unwrap());
        }
    }

    fn play(&mut self, backend: &mut Self::Backend) {
        if self.source.is_none() {
            self.source = backend.get_source();
        }

        if let Some(ref mut src) = self.source {
            src.handle.set_buffer(self.buffer.clone()).unwrap();
            src.handle.play();
        } else {
            error!("Failed to get source from backend!");
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

    fn get_position(&mut self) -> f32 {
        if let Some(ref mut src) = self.source {
            if src.handle.state() != alto::SourceState::Playing {
                return 0.0;
            }

            return src.handle.sec_offset() / self.length;
        }

        0.0
    }

    fn set_volume(&mut self, volume: f32) {
        if let Some(ref mut src) = self.source {
            src.handle.set_gain(volume);
        }
    }

    fn set_pitch(&mut self, pitch: f32) {
        if let Some(ref mut src) = self.source {
            src.handle.set_pitch(pitch);
        }
    }

    fn set_lowpass(&mut self, amount: f32) {
        if let Some(ref mut src) = self.source {
            if self.lowpass.is_none() {
                self.lowpass = Some(
                    src.handle
                        .context()
                        .new_filter::<alto::efx::LowpassFilter>()
                        .unwrap(),
                );
            }

            self.lowpass.as_mut().unwrap().set_gain(1.0);
            self.lowpass.as_mut().unwrap().set_gainhf(1.0 - amount);
            src.handle.set_direct_filter(self.lowpass.as_ref().unwrap());
        }
    }

    fn set_reverb(&mut self, reverb: &str) {
        if let Some(ref mut src) = self.source {
            let preset = match reverb_name_to_ref(reverb) {
                None => {
                    self.efx_slot = None;
                    self.reverb = None;
                    src.handle.clear_aux_send(0);
                    return;
                }
                Some(p) => p,
            };

            if self.efx_slot.is_none() {
                self.efx_slot = Some(src.handle.context().new_aux_effect_slot().unwrap());
                self.reverb = Some(
                    src.handle
                        .context()
                        .new_effect::<alto::efx::ReverbEffect>()
                        .unwrap(),
                );
            }

            info!("Setting preset {}: ...", reverb);
            self.reverb
                .as_mut()
                .unwrap()
                .set_preset(preset)
                .expect("Hello World2!");
            self.efx_slot
                .as_mut()
                .unwrap()
                .set_effect(self.reverb.as_ref().unwrap())
                .expect("Hello World1!");
            src.handle
                .set_aux_send(0, self.efx_slot.as_mut().unwrap())
                .expect("Hello World3!");
        }
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
        debug!(
            "Requesting source, {} sources available",
            self.sources.len()
        );

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

    fn reset_source(&self, source: &mut alto::StaticSource) {
        source.set_gain(1.0);
        source.set_pitch(1.0);
        source.clear_direct_filter();
        source.clear_aux_send(0);
    }

    fn free_source(&mut self, mut source: OpenALSource) {
        self.reset_source(&mut source.handle);
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
            }))
            .unwrap();

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
                i + 1,
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

        info!("Loaded {} samples at rate {}", samples.len(), sample_rate);

        let mut converted_samples = Vec::with_capacity(samples.len());
        for i in 0..samples.len() {
            converted_samples.push(alto::Mono { center: samples[i] });
        }

        let buf = self
            .context
            .new_buffer(converted_samples, sample_rate)
            .expect("Failed to get new buffer when loading file!");
        let buf = Arc::new(buf);

        Ok(Self::EntityData {
            buffer: buf,
            source: None,
            lowpass: None,
            efx_slot: None,
            reverb: None,
            length: samples.len() as f32 / sample_rate as f32
        })
    }

    fn set_volume(&mut self, volume: f32) {
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
