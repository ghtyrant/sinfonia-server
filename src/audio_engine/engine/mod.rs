mod messaging;

use rand::distributions::range::SampleRange;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, SystemTime};

use audio_engine::backends::base::{AudioBackend, AudioEntityData};
use audio_engine::messages::command;
use audio_engine::messages::response;
use error::SinfoniaGenericError;
use theme::Sound;
use utils::AsMillis;

fn get_random_value<T: PartialOrd + SampleRange>(val: (T, T)) -> T {
    if val.0 == val.1 {
        val.0
    } else {
        thread_rng().gen_range(val.0, val.1)
    }
}

pub struct AudioController<T: AudioBackend> {
    backend: T,
    receiver: Receiver<command::Command>,
    sender: Sender<response::Response>,
    sound_handles: HashMap<String, AudioEntity<T::EntityData>>,
    playing: bool,
    theme_loaded: bool,
    sound_library: PathBuf,
}

impl<T: AudioBackend> AudioController<T> {
    pub fn new(
        receiver: Receiver<command::Command>,
        sender: Sender<response::Response>,
        sound_library: PathBuf,
    ) -> Self {
        let backend = T::init();

        AudioController {
            backend,
            receiver,
            sender,
            sound_handles: HashMap::new(),
            playing: false,
            theme_loaded: false,
            sound_library,
        }
    }

    pub fn run(&mut self) -> Result<(), SinfoniaGenericError> {
        let mut quit = false;

        let clock = SystemTime::now();
        let mut last_update: u64 = clock.elapsed().unwrap().as_millis();

        while !quit {
            quit = match self.run_message_queue() {
                Ok(flag) => flag,
                Err(e) => {
                    error!("Failed to load file: {}", e);
                    false
                }
            };

            let time_elapsed = clock.elapsed().unwrap().as_millis() - last_update;

            for handle in &mut self.sound_handles.values_mut() {
                if handle.is_preview || self.playing && handle.sound.enabled {
                    handle.update(&mut self.backend, time_elapsed);
                }
            }

            last_update = clock.elapsed().unwrap().as_millis();
        }

        Ok(())
    }
}

#[derive(PartialEq, Debug)]
pub enum AudioEntityState {
    Virgin,
    Preview,
    PrepareRun,
    WaitingForStart,
    WaitingForTrigger,
    Starting,
    Playing,
    Repeat,
    Loop,
    Finished,
    Reset,
    Dead,
}

impl fmt::Display for AudioEntityState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct AudioEntity<O: AudioEntityData> {
    pub sound: Sound,
    pub object: O,

    pub parameters: AudioEntityParameters,
    pub is_triggered: bool,
    pub is_preview: bool,
}

pub struct AudioEntityParameters {
    pub state: AudioEntityState,
    pub next_play: Duration,
    pub repeats: u32,
    pub loops: u32,
}

impl AudioEntityParameters {
    pub fn new() -> Self {
        Self {
            state: AudioEntityState::Virgin,
            next_play: Duration::new(0, 0),
            repeats: 1,
            loops: 1,
        }
    }
}

impl<O: AudioEntityData> AudioEntity<O> {
    pub fn new(object: O, sound: Sound) -> Self {
        Self {
            sound,
            object,
            parameters: AudioEntityParameters::new(),
            is_triggered: false,
            is_preview: false,
        }
    }

    pub fn switch_state(&mut self, state: AudioEntityState) {
        debug!("Sound '{}' switching to state '{}'", self.sound.name, state);
        self.parameters.state = state;
    }

    pub fn is_in_state(&self, state: &AudioEntityState) -> bool {
        self.parameters.state == *state
    }

    pub fn pause(&mut self, flag: bool) {}

    pub fn reset(&mut self) {}

    pub fn update(&mut self, backend: &mut O::Backend, delta: u64) {
        match self.parameters.state {
            AudioEntityState::Virgin => {
                self.parameters.loops = get_random_value(self.sound.loop_count);

                info!(
                    "Will repeat this sound {}, and loop {} times!",
                    self.parameters.repeats, self.parameters.loops
                );

                if self.sound.trigger.is_some() && !self.is_preview {
                    self.switch_state(AudioEntityState::WaitingForTrigger);
                } else if self.is_preview {
                    self.switch_state(AudioEntityState::Starting);
                } else {
                    self.switch_state(AudioEntityState::PrepareRun);
                }
            }

            AudioEntityState::Preview => {
                self.switch_state(AudioEntityState::Reset);
            }

            AudioEntityState::Reset => {
                self.reset();

                self.switch_state(AudioEntityState::Virgin);
            }

            AudioEntityState::WaitingForTrigger => {
                if self.is_triggered {
                    self.switch_state(AudioEntityState::WaitingForStart);
                    self.is_triggered = false;
                }
            }

            AudioEntityState::PrepareRun => {
                self.parameters.repeats = get_random_value(self.sound.repeat_count);
                self.switch_state(AudioEntityState::WaitingForStart);
            }

            AudioEntityState::WaitingForStart => {
                // Decrease next_play down to 0.0s
                if self.parameters.next_play > Duration::new(0, 0) {
                    self.parameters.next_play = match self
                        .parameters
                        .next_play
                        .checked_sub(Duration::from_millis(delta))
                    {
                        Some(d) => d,
                        None => Duration::new(0, 0),
                    };
                }

                if self.parameters.next_play == Duration::new(0, 0) {
                    self.switch_state(AudioEntityState::Starting);
                }
            }

            AudioEntityState::Starting => {
                self.object.play(backend);

                self.switch_state(AudioEntityState::Playing);
            }

            AudioEntityState::Playing => {
                if self.object.is_playing() {
                    if self.is_preview {
                        self.is_preview = false;
                        self.switch_state(AudioEntityState::Virgin);
                    }
                } else {
                    if self.sound.trigger.is_some() && self.is_triggered {
                        info!("Sound {} cancelled!", self.sound.name);
                        self.object.stop(backend);

                        self.switch_state(AudioEntityState::Reset);
                        self.is_triggered = false;
                    } else {
                        self.switch_state(AudioEntityState::Repeat);
                    }
                }
            }

            AudioEntityState::Repeat => {
                self.parameters.repeats -= 1;

                if self.parameters.repeats > 0 {
                    self.parameters.next_play =
                        Duration::from_millis(get_random_value(self.sound.repeat_delay));
                    info!("Repeats are {}", self.parameters.repeats);

                    self.switch_state(AudioEntityState::WaitingForStart);
                } else {
                    self.switch_state(AudioEntityState::Loop);
                }
            }

            AudioEntityState::Loop => {
                if !self.sound.loops_forever {
                    self.parameters.loops -= 1;
                }

                if self.parameters.loops > 0 || self.sound.loops_forever {
                    self.parameters.next_play =
                        Duration::from_millis(get_random_value(self.sound.loop_delay));
                    info!("Repeats are {}", self.parameters.repeats);

                    self.switch_state(AudioEntityState::PrepareRun);
                } else {
                    self.switch_state(AudioEntityState::Finished);
                }
            }

            AudioEntityState::Finished => {
                info!("Sound {} finished!", self.sound.name);

                if self.sound.trigger.is_some() {
                    self.switch_state(AudioEntityState::Reset);
                } else {
                    self.switch_state(AudioEntityState::Dead);
                }
            }

            AudioEntityState::Dead => {
                //self.parameters.channel = None;
            }
        }
    }
}

pub fn start_audio_controller<T: AudioBackend>(
    receiver: Receiver<command::Command>,
    sender: Sender<response::Response>,
    sound_library: PathBuf,
) {
    let mut audio_ctrl: AudioController<T> = AudioController::new(receiver, sender, sound_library);

    match audio_ctrl.run() {
        Ok(()) => info!("AudioController exited ok"),
        Err(e) => error!("Error while running AudioController: {}", e),
    };
}
