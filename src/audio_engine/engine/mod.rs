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

fn get_random_value<T: PartialOrd + SampleRange + fmt::Display>(val: (T, T)) -> T {
    if val.0 == val.1 {
        val.0
    } else {
        info!("Get random value for {}, {}, are not equal!", val.0, val.1);
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
    theme: Option<String>,
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
            theme: None,
            sound_library,
        }
    }

    pub fn run(&mut self) -> Result<(), SinfoniaGenericError> {
        let mut quit = false;

        let clock = SystemTime::now();
        let mut last_update: u64 = clock.elapsed().unwrap().as_millis() as u64;

        while !quit {
            quit = match self.run_message_queue() {
                Ok(flag) => flag,
                Err(e) => {
                    error!("Error running message queue: {}", e);
                    false
                }
            };

            let time_elapsed = clock.elapsed().unwrap().as_millis() as u64 - last_update;

            for handle in &mut self.sound_handles.values_mut() {
                if handle.is_preview || self.playing && handle.sound.enabled {
                    handle.update(&mut self.backend, time_elapsed);
                }
            }

            last_update = clock.elapsed().unwrap().as_millis() as u64;
        }

        info!("AudioEngine stopped");

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
    pub fade_in: f32,
    pub max_volume: f32,
}

impl AudioEntityParameters {
    pub fn new() -> Self {
        Self {
            state: AudioEntityState::Virgin,
            next_play: Duration::new(0, 0),
            repeats: 0,
            loops: 1,
            fade_in: 0.0,
            max_volume: 1.0,
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

    pub fn pause(&mut self, flag: bool) {
        self.object.pause();
    }

    pub fn play(&mut self, backend: &mut O::Backend) {
        self.object.play(backend);
    }

    pub fn stop(&mut self, backend: &mut O::Backend) {
        self.object.stop(backend);
    }

    pub fn update(&mut self, backend: &mut O::Backend, delta: u64) {
        match self.parameters.state {
            // Initial state every new sound is in
            AudioEntityState::Virgin => {
                self.parameters.next_play =
                    Duration::from_millis(get_random_value(self.sound.loop_delay));
                self.parameters.loops = get_random_value(self.sound.loop_count);

                if self.sound.trigger.is_some() && !self.is_preview {
                    self.switch_state(AudioEntityState::WaitingForTrigger);
                } else if self.is_preview {
                    self.switch_state(AudioEntityState::PrepareRun);
                } else {
                    self.switch_state(AudioEntityState::PrepareRun);
                }
            }

            AudioEntityState::Preview => {
                self.switch_state(AudioEntityState::Reset);
            }

            AudioEntityState::Reset => {
                self.stop(backend);

                self.switch_state(AudioEntityState::Virgin);
            }

            // The sound is waiting for an external trigger to start
            AudioEntityState::WaitingForTrigger => {
                if self.is_triggered {
                    self.switch_state(AudioEntityState::WaitingForStart);
                    self.is_triggered = false;
                }
            }

            // Prepare sound parameters before a run starts
            // e.g. determine the number of times the sound will be repeated
            AudioEntityState::PrepareRun => {
                self.parameters.repeats = get_random_value(self.sound.repeat_count);
                info!(
                    "Will repeat this sound {}, and loop {} times!",
                    self.parameters.repeats, self.parameters.loops
                );

                if self.is_preview {
                    self.switch_state(AudioEntityState::Starting);
                } else {
                    self.switch_state(AudioEntityState::WaitingForStart);
                }
            }

            // Wait for specified delay between loops
            AudioEntityState::WaitingForStart => {
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

            // Start playing the sound
            AudioEntityState::Starting => {
                self.play(backend);
                self.parameters.max_volume = get_random_value(self.sound.volume);

                let mut pitch = -1.0;
                if self.sound.pitch_enabled {
                    pitch = get_random_value(self.sound.pitch);
                    self.object.set_pitch(pitch);
                }

                let mut lowpass = -1.0;
                if self.sound.lowpass_enabled {
                    lowpass = get_random_value(self.sound.lowpass);
                    self.object.set_lowpass(lowpass);
                }

                let mut fade_in = 0.0;
                if self.sound.fade_in_enabled {
                    fade_in = get_random_value(self.sound.fade_in);
                    self.parameters.fade_in = fade_in;
                }

                self.object.set_reverb(self.sound.reverb.as_ref());

                info!(
                    "Going to play {} at volume {}, pitch {}, lowpass {}, with reverb {}, fade in until {}",
                    self.sound.name, self.parameters.max_volume, pitch, lowpass, self.sound.reverb, self.parameters.fade_in
                );

                self.switch_state(AudioEntityState::Playing);
            }

            // Wait until the sound is done
            AudioEntityState::Playing => {
                let mut volume = self.parameters.max_volume;

                if self.object.get_position() < self.parameters.fade_in {
                    volume = (1.0
                        - (self.parameters.fade_in - self.object.get_position())
                            / self.parameters.fade_in)
                        * self.parameters.max_volume;
                }

                self.object.set_volume(volume);

                if !self.object.is_playing() {
                    if self.sound.trigger.is_some() && self.is_triggered {
                        info!("Sound {} cancelled!", self.sound.name);
                        self.stop(backend);

                        self.switch_state(AudioEntityState::Reset);
                        self.is_triggered = false;
                    } else {
                        self.switch_state(AudioEntityState::Repeat);
                    }
                }
            }

            // If the sound needs to be repeated, do it here
            AudioEntityState::Repeat => {
                if self.parameters.repeats > 0 {
                    self.parameters.repeats -= 1;
                    self.parameters.next_play =
                        Duration::from_millis(get_random_value(self.sound.repeat_delay));
                    info!("Repeats are {}", self.parameters.repeats);

                    self.switch_state(AudioEntityState::WaitingForStart);
                } else {
                    // If this run was a preview, exit preview mode and switch back to being a virgin
                    if self.is_preview {
                        self.is_preview = false;
                        self.switch_state(AudioEntityState::Virgin);
                    } else {
                        // Otherwise go back to looping
                        self.switch_state(AudioEntityState::Loop);
                    }
                }
            }

            // If the sound needs to be looped, do it here
            AudioEntityState::Loop => {
                // Stop the sound for now to free up resources
                self.stop(backend);

                if self.parameters.loops > 0 || self.sound.loop_forever {
                    if !self.sound.loop_forever {
                        self.parameters.loops -= 1;
                    }

                    self.parameters.next_play =
                        Duration::from_millis(get_random_value(self.sound.loop_delay));

                    self.switch_state(AudioEntityState::PrepareRun);
                } else {
                    self.switch_state(AudioEntityState::Finished);
                }
            }

            // Finished all repeats and loops, sound is done
            AudioEntityState::Finished => {
                info!("Sound {} finished!", self.sound.name);

                if self.sound.trigger.is_some() {
                    self.switch_state(AudioEntityState::Reset);
                } else {
                    self.switch_state(AudioEntityState::Dead);
                }
            }

            // Once we are here, the sound is dead
            AudioEntityState::Dead => {}
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
