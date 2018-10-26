use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, SystemTime};

use rfmod;

use error::AudioControllerError;
use theme::{FuncList, Sound, Theme, FUNC_TYPE_FINISH, FUNC_TYPE_START, FUNC_TYPE_UPDATE};
use utils::AsMillis;

#[derive(Serialize)]
pub struct AudioControllerStatus {
    playing: bool,
    theme_loaded: bool,
    sounds_playing: Vec<String>,
}

pub struct AudioControllerLoadThemeResponse {
    pub success: bool,
}

pub struct AudioControllerTriggerResponse {
    pub trigger_found: bool,
}

#[derive(Serialize)]
pub struct AudioControllerSoundLibrary {
    pub sounds: Vec<String>,
}

#[derive(Serialize)]
pub struct AudioControllerDriverList {
    pub drivers: Vec<(i32, String)>,
}

#[derive(Serialize)]
pub struct AudioControllerDriver {
    pub id: i32,
}

const SUPPORTED_AUDIO_FILES: [&str; 14] = [
    "aiff", "asf", "dls", "flac", "fsb", "it", "midi", "mod", "mp3", "mp4", "ogg", "wav", "xm",
    "xma",
];

pub enum AudioControllerMessage {
    Quit,
    Play,
    PreviewSound {
        sound: String,
    },
    Pause,
    LoadTheme {
        theme: Theme,
        response_sender: Sender<AudioControllerLoadThemeResponse>,
    },
    Trigger {
        sound: String,
        response_sender: Sender<AudioControllerTriggerResponse>,
    },
    GetStatus {
        response_sender: Sender<AudioControllerStatus>,
    },
    GetSoundLibrary {
        response_sender: Sender<AudioControllerSoundLibrary>,
    },
    Volume {
        value: f32,
    },
    GetDriver {
        response_sender: Sender<AudioControllerDriver>,
    },
    GetDriverList {
        response_sender: Sender<AudioControllerDriverList>,
    },
    SetDriver {
        id: i32,
    },
}

pub struct AudioController {
    fmod: rfmod::Sys,
    master_channel_group: rfmod::ChannelGroup,
    receiver: Receiver<AudioControllerMessage>,
    sound_handles: HashMap<String, SoundHandle>,
    playing: bool,
    theme_loaded: bool,
    sound_library: PathBuf,
}

impl AudioController {
    pub fn new(receiver: Receiver<AudioControllerMessage>, sound_library: PathBuf) -> Self {
        let fmod = match rfmod::Sys::new() {
            Ok(f) => f,
            Err(e) => {
                panic!("Error code : {:?}", e);
            }
        };

        match fmod.init_with_parameters(100, rfmod::InitFlag(rfmod::INIT_NORMAL)) {
            rfmod::Status::Ok => {}
            e => {
                panic!("FmodSys.init failed : {:?}", e);
            }
        };

        let master_channel_group = match fmod.get_master_channel_group() {
            Ok(group) => group,
            Err(e) => {
                panic!("Failed to get master channel group: {:?}", e);
            }
        };

        AudioController {
            receiver,
            fmod,
            master_channel_group,
            sound_handles: HashMap::new(),
            playing: false,
            theme_loaded: false,
            sound_library,
        }
    }

    pub fn run(&mut self) -> Result<(), AudioControllerError> {
        let mut quit = false;
        let timeout = Duration::from_millis(50);
        let clock = SystemTime::now();
        let mut last_update: u64 = clock.elapsed().unwrap().as_millis();

        while !quit {
            if let Ok(msg) = self.receiver.recv_timeout(timeout) {
                match msg {
                    AudioControllerMessage::Quit => {
                        quit = true;
                    }

                    AudioControllerMessage::Pause => {
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

                    AudioControllerMessage::Play => {
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

                    AudioControllerMessage::PreviewSound { sound } => {
                        if let Some(handle) = self.sound_handles.get_mut(&sound) {
                            handle.is_preview = true;
                            handle.switch_state(SoundHandleState::Preview);
                        }
                    }

                    AudioControllerMessage::LoadTheme {
                        theme,
                        response_sender,
                    } => {
                        self.sound_handles.clear();

                        for sound in theme.sounds {
                            self.sound_handles.insert(
                                sound.name.clone(),
                                SoundHandle::new(&self.fmod, sound, &self.sound_library),
                            );
                        }

                        self.theme_loaded = true;

                        try!(Ok(response_sender
                            .send(AudioControllerLoadThemeResponse { success: true })));

                        info!("Theme loaded!")
                    }

                    AudioControllerMessage::Trigger {
                        sound,
                        response_sender,
                    } => {
                        let mut success = false;
                        if let Some(handle) = self.sound_handles.get_mut(&sound) {
                            info!("Received trigger for sound '{}'!", sound);
                            handle.is_triggered = !handle.is_triggered;
                            success = true;
                        } else {
                            error!("Received trigger for unknown sound '{}'!", sound);
                        }

                        try!(Ok(response_sender.send(AudioControllerTriggerResponse {
                            trigger_found: success,
                        })));
                    }

                    AudioControllerMessage::GetStatus { response_sender } => {
                        let mut playing: Vec<String> = Vec::new();
                        for (name, handle) in &self.sound_handles {
                            if handle.is_in_state(&SoundHandleState::Playing) {
                                playing.push(name.to_string());
                            }
                        }

                        try!(Ok(response_sender.send(AudioControllerStatus {
                            playing: self.playing,
                            theme_loaded: self.theme_loaded,
                            sounds_playing: playing,
                        })));
                    }

                    AudioControllerMessage::GetSoundLibrary { response_sender } => {
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

                        try!(Ok(
                            response_sender.send(AudioControllerSoundLibrary { sounds: lib })
                        ));
                    }

                    AudioControllerMessage::Volume { value } => {
                        self.master_channel_group.set_volume(value);
                    }

                    AudioControllerMessage::GetDriverList { response_sender } => {
                        let mut drivers: Vec<(i32, String)> = Vec::new();

                        let num_drivers = self
                            .fmod
                            .get_num_drivers()
                            .expect("Failed to enumerate drivers.");
                        for i in 0..num_drivers {
                            match self.fmod.get_driver_info(i, 256usize) {
                                Ok((_, name)) => drivers.push((i, name)),
                                Err(e) => {
                                    error!("get_driver_info error: {:?}", e);
                                }
                            };
                        }

                        try!(Ok(
                            response_sender.send(AudioControllerDriverList { drivers })
                        ));
                    }

                    AudioControllerMessage::GetDriver { response_sender } => {
                        let id = match self.fmod.get_driver() {
                            Ok(id) => id,
                            Err(e) => {
                                error!("get_driver error: {:?}", e);
                                0
                            }
                        };

                        try!(Ok(response_sender.send(AudioControllerDriver { id })));
                    }

                    AudioControllerMessage::SetDriver { id } => {
                        info!("Changing driver to {}!", id);
                        self.fmod.set_driver(id);
                    }
                }
            };

            let time_elapsed = clock.elapsed().unwrap().as_millis() - last_update;

            for (_, handle) in &mut self.sound_handles {
                if handle.is_preview || self.playing && !handle.sound.is_disabled {
                    handle.update(&self.fmod, time_elapsed);
                }
            }

            last_update = clock.elapsed().unwrap().as_millis();
        }

        Ok(())
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

pub struct SoundHandle {
    pub sound: Sound,
    pub handle: rfmod::Sound,

    pub parameters: SoundHandleParameters,
    pub is_triggered: bool,
    pub is_preview: bool,
}

pub struct SoundHandleParameters {
    pub state: SoundHandleState,
    pub next_play: Duration,
    pub channel: Option<rfmod::Channel>,
    pub dsps: HashMap<String, rfmod::Dsp>,
    pub should_loop: bool,
}

impl SoundHandleParameters {
    pub fn new() -> Self {
        Self {
            state: SoundHandleState::Virgin,
            next_play: Duration::new(0, 0),
            channel: None,
            dsps: HashMap::new(),
            should_loop: false,
        }
    }
}

fn reset_states(funcs: &mut FuncList) {
    for func in funcs.iter_mut() {
        (*func).reset_state();
    }
}

impl SoundHandle {
    pub fn new(fmod: &rfmod::Sys, sound: Sound, base_path: &PathBuf) -> Self {
        let mut full_path: PathBuf = PathBuf::from(base_path);
        full_path.push(sound.file_path.clone());
        let handle = match fmod.create_sound(&full_path.to_str().unwrap(), None, None) {
            Ok(s) => s,
            Err(err) => {
                panic!("Sys::create_sound() failed : {:?}", err);
            }
        };

        Self {
            sound,
            handle,
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
        if self.parameters.channel.is_none() {
            return;
        }

        self.parameters.channel.as_ref().unwrap().set_paused(flag);
    }

    pub fn reset(&mut self) {
        self.parameters.channel = None;

        reset_states(&mut self.sound.funcs[FUNC_TYPE_START]);
        reset_states(&mut self.sound.funcs[FUNC_TYPE_UPDATE]);
        reset_states(&mut self.sound.funcs[FUNC_TYPE_FINISH]);
    }

    pub fn update(&mut self, fmod: &rfmod::Sys, delta: u64) {
        fn run_funcs(funcs: &mut FuncList, parameters: &mut SoundHandleParameters) {
            for func in funcs.iter_mut() {
                (*func).execute(parameters);
            }
        }

        match self.parameters.state {
            SoundHandleState::Virgin => match self.handle.play() {
                Ok(chan) => {
                    info!("Preparing sound {} for being played", self.sound.name);
                    self.parameters.channel = Some(chan);
                    self.parameters.channel.as_ref().unwrap().set_paused(true);

                    self.parameters.dsps.insert(
                        "echo".into(),
                        match fmod.create_DSP_by_type(rfmod::DspType::Echo) {
                            Ok(dsp) => {
                                dsp.set_bypass(true);
                                dsp
                            }
                            Err(e) => panic!("Failed to create Echo DSP: {:?}", e),
                        },
                    );

                    self.parameters.dsps.insert(
                        "lowpass".into(),
                        match fmod.create_DSP_by_type(rfmod::DspType::LowPass) {
                            Ok(dsp) => {
                                dsp.set_bypass(true);
                                dsp
                            }
                            Err(e) => panic!("Failed to create LowPass DSP: {:?}", e),
                        },
                    );

                    self.parameters.dsps.insert(
                        "reverb".into(),
                        match fmod.create_DSP_by_type(rfmod::DspType::SFXReverb) {
                            Ok(dsp) => {
                                dsp.set_bypass(true);
                                dsp
                            }
                            Err(e) => panic!("Failed to create Reverb DSP: {:?}", e),
                        },
                    );

                    self.parameters
                        .channel
                        .as_ref()
                        .unwrap()
                        .add_DSP(&self.parameters.dsps["echo"])
                        .expect("Failed to add Echo DSP!");
                    self.parameters
                        .channel
                        .as_ref()
                        .unwrap()
                        .add_DSP(&self.parameters.dsps["lowpass"])
                        .expect("Failed to add LowPass DSP!");
                    self.parameters
                        .channel
                        .as_ref()
                        .unwrap()
                        .add_DSP(&self.parameters.dsps["reverb"])
                        .expect("Failed to add LowPass DSP!");

                    run_funcs(&mut self.sound.funcs[FUNC_TYPE_START], &mut self.parameters);

                    if self.sound.needs_trigger && !self.is_preview {
                        self.switch_state(SoundHandleState::WaitingForTrigger);
                    } else if self.is_preview {
                        self.switch_state(SoundHandleState::Starting);
                    } else {
                        self.switch_state(SoundHandleState::WaitingForStart);
                    }
                }
                Err(err) => {
                    error!("Error playing sound: {:?}", err);
                }
            },

            SoundHandleState::Preview => {
                if self.parameters.channel.is_some() {
                    self.parameters.channel.as_ref().unwrap().stop();
                }

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
                self.parameters.channel.as_ref().unwrap().set_paused(false);
                self.switch_state(SoundHandleState::Playing);
            }

            SoundHandleState::Playing => {
                match self.parameters.channel.as_ref().unwrap().is_playing() {
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
                                self.parameters.channel.as_ref().unwrap().stop();

                                self.switch_state(SoundHandleState::Reset);
                                self.is_triggered = false;
                            }
                        }
                    }
                    Err(err) => {
                        error!("Error querying channel: {:?}", err);
                        self.parameters.channel = None;
                    }
                }
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
                self.parameters.channel = None;
            }
        }
    }
}

pub fn start_audio_controller(receiver: Receiver<AudioControllerMessage>, sound_library: PathBuf) {
    let mut audio_ctrl: AudioController = AudioController::new(receiver, sound_library);

    match audio_ctrl.run() {
        Ok(()) => info!("AudioController exited ok"),
        Err(e) => error!("Error while running AudioController: {}", e),
    };
}
