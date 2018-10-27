mod messaging;

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, SystemTime};

use audio_engine::backends::base::{AudioBackend, AudioEntityData};
use audio_engine::messages::command;
use audio_engine::messages::response;
use error::AudioControllerError;
use theme::{FuncList, Sound, Theme, FUNC_TYPE_FINISH, FUNC_TYPE_START, FUNC_TYPE_UPDATE};
use utils::AsMillis;

pub struct AudioController<T: AudioBackend> {
    backend: T,
    receiver: Receiver<command::Command>,
    sound_handles: HashMap<String, AudioEntity<T::AudioBackendEntityData>>,
    playing: bool,
    theme_loaded: bool,
    sound_library: PathBuf,
}

impl<T: AudioBackend> AudioController<T> {
    pub fn new(receiver: Receiver<command::Command>, sound_library: PathBuf) -> Self {
        let backend = T::init();

        AudioController {
            backend,
            receiver,
            sound_handles: HashMap::new(),
            playing: false,
            theme_loaded: false,
            sound_library,
        }
    }

    pub fn run(&mut self) -> Result<(), AudioControllerError> {
        let mut quit = false;

        let clock = SystemTime::now();
        let mut last_update: u64 = clock.elapsed().unwrap().as_millis();

        while !quit {
            quit = self.run_message_queue().unwrap();

            let time_elapsed = clock.elapsed().unwrap().as_millis() - last_update;

            for handle in &mut self.sound_handles.values_mut() {
                if handle.is_preview || self.playing && !handle.sound.is_disabled {
                    handle.update(time_elapsed);
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
    WaitingForStart,
    WaitingForTrigger,
    Starting,
    Playing,
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
    pub should_loop: bool,
}

impl AudioEntityParameters {
    pub fn new() -> Self {
        Self {
            state: AudioEntityState::Virgin,
            next_play: Duration::new(0, 0),
            should_loop: false,
        }
    }
}

fn reset_states(funcs: &mut FuncList) {
    for func in funcs.iter_mut() {
        (*func).reset_state();
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
        /*if self.parameters.channel.is_none() {
            return;
        }*/

        //self.parameters.channel.as_ref().unwrap().set_paused(flag);
    }

    pub fn reset(&mut self) {
        //self.parameters.channel = None;

        reset_states(&mut self.sound.funcs[FUNC_TYPE_START]);
        reset_states(&mut self.sound.funcs[FUNC_TYPE_UPDATE]);
        reset_states(&mut self.sound.funcs[FUNC_TYPE_FINISH]);
    }

    pub fn update(&mut self, delta: u64) {
        fn run_funcs(funcs: &mut FuncList, parameters: &mut AudioEntityParameters) {
            for func in funcs.iter_mut() {
                (*func).execute(parameters);
            }
        }

        match self.parameters.state {
            AudioEntityState::Virgin => {
                self.object.play();
                run_funcs(&mut self.sound.funcs[FUNC_TYPE_START], &mut self.parameters);

                if self.sound.needs_trigger && !self.is_preview {
                    self.switch_state(AudioEntityState::WaitingForTrigger);
                } else if self.is_preview {
                    self.switch_state(AudioEntityState::Starting);
                } else {
                    self.switch_state(AudioEntityState::WaitingForStart);
                }
            }

            AudioEntityState::Preview => {
                /*if self.parameters.channel.is_some() {
                    self.parameters.channel.as_ref().unwrap().stop();
                }*/

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
                //self.parameters.channel.as_ref().unwrap().set_paused(false);
                self.switch_state(AudioEntityState::Playing);
            }

            AudioEntityState::Playing => {
                /*match self.parameters.channel.as_ref().unwrap().is_playing() {
                    Ok(playing) => {
                        if !playing {
                            // Sound has finished
                            if self.is_preview {
                                self.is_preview = false;
                                self.switch_state(AudioEntityState::Virgin);
                            } else {
                                self.switch_state(AudioEntityState::Finished);
                            }
                        } else {
                            // Run update functions
                            run_funcs(
                                &mut self.sound.funcs[FUNC_TYPE_UPDATE],
                                &mut self.parameters,
                            );

                            // If we are playing and get triggered, we should stop
                            if self.sound.needs_trigger && self.is_triggered {
                                info!("Sound {} cancelled!", self.sound.name);
                                //self.parameters.channel.as_ref().unwrap().stop();

                                self.switch_state(AudioEntityState::Reset);
                                self.is_triggered = false;
                            }
                        }
                    }
                    Err(err) => {
                        error!("Error querying channel: {:?}", err);
                        self.parameters.channel = None;
                    }
                }*/
            }

            AudioEntityState::Finished => {
                info!("Sound {} finished!", self.sound.name);

                if self.sound.needs_trigger {
                    self.switch_state(AudioEntityState::Reset);
                } else {
                    self.switch_state(AudioEntityState::Dead);
                }

                run_funcs(
                    &mut self.sound.funcs[FUNC_TYPE_FINISH],
                    &mut self.parameters,
                );

                if self.parameters.should_loop {
                    self.switch_state(AudioEntityState::Reset);
                    self.parameters.should_loop = false;

                    // If we need a trigger but still want to get looped, just trigger again
                    if self.sound.needs_trigger {
                        self.is_triggered = true;
                    }
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
    sound_library: PathBuf,
) {
    let mut audio_ctrl: AudioController<T> = AudioController::new(receiver, sound_library);

    match audio_ctrl.run() {
        Ok(()) => info!("AudioController exited ok"),
        Err(e) => error!("Error while running AudioController: {}", e),
    };
}
