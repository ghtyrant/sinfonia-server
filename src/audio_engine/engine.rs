use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, SystemTime};

use audio_engine::backends::base::{AudioBackend, AudioObject};
use audio_engine::messages::*;
use error::AudioControllerError;
use theme::{FuncList, Sound, Theme, FUNC_TYPE_FINISH, FUNC_TYPE_START, FUNC_TYPE_UPDATE};
use utils::AsMillis;

// TODO This information should come from our loaders
const SUPPORTED_AUDIO_FILES: [&str; 5] = ["aiff", "flac", "midi", "ogg", "wav"];

pub struct AudioController<T: AudioBackend> {
    backend: T,
    receiver: Receiver<AudioControllerMessage>,
    sound_handles: HashMap<String, SoundHandle<T::AudioBackendObject>>,
    playing: bool,
    theme_loaded: bool,
    sound_library: PathBuf,
}

impl<T: AudioBackend> AudioController<T> {
    pub fn new(receiver: Receiver<AudioControllerMessage>, sound_library: PathBuf) -> Self {
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

    fn handle_pause(&mut self) {
        if self.theme_loaded {
            for (_, handle) in &mut self.sound_handles {
                if handle.is_in_state(&SoundHandleState::Playing) {
                    handle.pause(true);
                }
            }

            self.playing = false;

            info!("Paused!")
        } else {
            debug!("No theme loaded, not pausing ...")
        }
    }

    fn handle_play(&mut self) {
        if self.theme_loaded {
            for (_, handle) in &mut self.sound_handles {
                if handle.is_in_state(&SoundHandleState::Playing) {
                    handle.pause(false);
                }
            }

            self.playing = true;

            info!("Playing!")
        } else {
            debug!("No theme loaded, not playing ...")
        }
    }

    fn handle_preview_sound(&mut self, sound: String) {
        if let Some(handle) = self.sound_handles.get_mut(&sound) {
            handle.is_preview = true;
            handle.switch_state(SoundHandleState::Preview);
        }
    }

    fn handle_load_theme(
        &mut self,
        theme: Theme,
        response_sender: Sender<AudioControllerLoadThemeResponse>,
    ) {
        self.sound_handles.clear();

        for sound in theme.sounds {
            let mut full_path: PathBuf = PathBuf::from(&self.sound_library);
            full_path.push(sound.file_path.clone());
            let object = self.backend.load_object(&full_path);

            info!("Loading file {} ...", &full_path.to_str().unwrap());

            self.sound_handles
                .insert(sound.name.clone(), SoundHandle::new(object, sound));
        }

        self.theme_loaded = true;

        response_sender.send(AudioControllerLoadThemeResponse { success: true });

        info!("Theme loaded!")
    }

    fn handle_trigger(
        &mut self,
        sound: String,
        response_sender: Sender<AudioControllerTriggerResponse>,
    ) {
        let mut success = false;
        if let Some(handle) = self.sound_handles.get_mut(&sound) {
            info!("Received trigger for sound '{}'!", sound);
            handle.is_triggered = !handle.is_triggered;
            success = true;
        } else {
            error!("Received trigger for unknown sound '{}'!", sound);
        }

        response_sender.send(AudioControllerTriggerResponse {
            trigger_found: success,
        });
    }

    fn handle_get_status(&mut self, response_sender: Sender<AudioControllerStatus>) {
        let mut playing: Vec<String> = Vec::new();
        for (name, handle) in &self.sound_handles {
            if handle.is_in_state(&SoundHandleState::Playing) {
                playing.push(name.to_string());
            }
        }

        response_sender.send(AudioControllerStatus {
            playing: self.playing,
            theme_loaded: self.theme_loaded,
            sounds_playing: playing,
        });
    }

    fn handle_get_sound_library(&mut self, response_sender: Sender<AudioControllerSoundLibrary>) {
        let mut lib: Vec<String> = Vec::new();
        for entry in self.sound_library.read_dir().expect("read_dir call failed") {
            if let Ok(entry) = entry {
                if let Some(extension) = entry.path().extension() {
                    if SUPPORTED_AUDIO_FILES.iter().any(|&ext| ext == extension) {
                        lib.push(entry.file_name().to_str().unwrap().into());
                    }
                }
            }
        }

        response_sender.send(AudioControllerSoundLibrary { sounds: lib });
    }

    fn handle_volume(&mut self, value: f32) {
        self.backend.set_volume(value);
    }

    fn handle_get_driver_list(&mut self, response_sender: Sender<AudioControllerDriverList>) {
        let mut drivers: Vec<(i32, String)> = Vec::new();

        self.backend
            .get_output_devices()
            .into_iter()
            .for_each(|d| drivers.push((0, d)));

        response_sender.send(AudioControllerDriverList { drivers });
    }

    fn handle_get_driver(&mut self, response_sender: Sender<AudioControllerDriver>) {
        response_sender.send(AudioControllerDriver {
            id: self.backend.get_current_output_device(),
        });
    }

    fn handle_set_driver(&mut self, id: i32) {
        self.backend.set_current_output_device(id);
    }

    fn run_message_queue(&mut self) -> Result<bool, AudioControllerError> {
        let timeout = Duration::from_millis(50);

        if let Ok(msg) = self.receiver.recv_timeout(timeout) {
            match msg {
                AudioControllerMessage::Quit => return Ok(true),
                AudioControllerMessage::Pause => self.handle_pause(),
                AudioControllerMessage::Play => self.handle_play(),
                AudioControllerMessage::PreviewSound { sound } => self.handle_preview_sound(sound),
                AudioControllerMessage::LoadTheme {
                    theme,
                    response_sender,
                } => self.handle_load_theme(theme, response_sender),

                AudioControllerMessage::Trigger {
                    sound,
                    response_sender,
                } => self.handle_trigger(sound, response_sender),

                AudioControllerMessage::GetStatus { response_sender } => {
                    self.handle_get_status(response_sender)
                }

                AudioControllerMessage::GetSoundLibrary { response_sender } => {
                    self.handle_get_sound_library(response_sender)
                }

                AudioControllerMessage::Volume { value } => self.handle_volume(value),

                AudioControllerMessage::GetDriverList { response_sender } => {
                    self.handle_get_driver_list(response_sender)
                }

                AudioControllerMessage::GetDriver { response_sender } => {
                    self.handle_get_driver(response_sender)
                }

                AudioControllerMessage::SetDriver { id } => self.handle_set_driver(id),
            }
        };

        Ok(false)
    }
}

#[derive(PartialEq, Debug)]
pub enum SoundHandleState {
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

impl fmt::Display for SoundHandleState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct SoundHandle<O: AudioObject> {
    pub sound: Sound,
    pub object: O,

    pub parameters: SoundHandleParameters,
    pub is_triggered: bool,
    pub is_preview: bool,
}

pub struct SoundHandleParameters {
    pub state: SoundHandleState,
    pub next_play: Duration,
    pub should_loop: bool,
}

impl SoundHandleParameters {
    pub fn new() -> Self {
        Self {
            state: SoundHandleState::Virgin,
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

impl<O: AudioObject> SoundHandle<O> {
    pub fn new(object: O, sound: Sound) -> Self {
        Self {
            sound,
            object,
            parameters: SoundHandleParameters::new(),
            is_triggered: false,
            is_preview: false,
        }
    }

    pub fn switch_state(&mut self, state: SoundHandleState) {
        debug!("Sound '{}' switching to state '{}'", self.sound.name, state);
        self.parameters.state = state;
    }

    pub fn is_in_state(&self, state: &SoundHandleState) -> bool {
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
        fn run_funcs(funcs: &mut FuncList, parameters: &mut SoundHandleParameters) {
            for func in funcs.iter_mut() {
                (*func).execute(parameters);
            }
        }

        match self.parameters.state {
            SoundHandleState::Virgin => {
                self.object.play();
                run_funcs(&mut self.sound.funcs[FUNC_TYPE_START], &mut self.parameters);

                if self.sound.needs_trigger && !self.is_preview {
                    self.switch_state(SoundHandleState::WaitingForTrigger);
                } else if self.is_preview {
                    self.switch_state(SoundHandleState::Starting);
                } else {
                    self.switch_state(SoundHandleState::WaitingForStart);
                }
            }

            SoundHandleState::Preview => {
                /*if self.parameters.channel.is_some() {
                    self.parameters.channel.as_ref().unwrap().stop();
                }*/

                self.switch_state(SoundHandleState::Reset);
            }

            SoundHandleState::Reset => {
                self.reset();

                self.switch_state(SoundHandleState::Virgin);
            }

            SoundHandleState::WaitingForTrigger => {
                if self.is_triggered {
                    self.switch_state(SoundHandleState::WaitingForStart);
                    self.is_triggered = false;
                }
            }

            SoundHandleState::WaitingForStart => {
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
                    self.switch_state(SoundHandleState::Starting);
                }
            }

            SoundHandleState::Starting => {
                //self.parameters.channel.as_ref().unwrap().set_paused(false);
                self.switch_state(SoundHandleState::Playing);
            }

            SoundHandleState::Playing => {
                /*match self.parameters.channel.as_ref().unwrap().is_playing() {
                    Ok(playing) => {
                        if !playing {
                            // Sound has finished
                            if self.is_preview {
                                self.is_preview = false;
                                self.switch_state(SoundHandleState::Virgin);
                            } else {
                                self.switch_state(SoundHandleState::Finished);
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

                                self.switch_state(SoundHandleState::Reset);
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

            SoundHandleState::Finished => {
                info!("Sound {} finished!", self.sound.name);

                if self.sound.needs_trigger {
                    self.switch_state(SoundHandleState::Reset);
                } else {
                    self.switch_state(SoundHandleState::Dead);
                }

                run_funcs(
                    &mut self.sound.funcs[FUNC_TYPE_FINISH],
                    &mut self.parameters,
                );

                if self.parameters.should_loop {
                    self.switch_state(SoundHandleState::Reset);
                    self.parameters.should_loop = false;

                    // If we need a trigger but still want to get looped, just trigger again
                    if self.sound.needs_trigger {
                        self.is_triggered = true;
                    }
                }
            }

            SoundHandleState::Dead => {
                //self.parameters.channel = None;
            }
        }
    }
}

pub fn start_audio_controller<T: AudioBackend>(
    receiver: Receiver<AudioControllerMessage>,
    sound_library: PathBuf,
) {
    let mut audio_ctrl: AudioController<T> = AudioController::new(receiver, sound_library);

    match audio_ctrl.run() {
        Ok(()) => info!("AudioController exited ok"),
        Err(e) => error!("Error while running AudioController: {}", e),
    };
}
