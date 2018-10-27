use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

use audio_engine::backends::base::{AudioBackend, AudioEntityData};
use audio_engine::engine::AudioEntity;
use audio_engine::engine::{AudioController, AudioEntityState};
use audio_engine::messages::{command, response};
use error::AudioControllerError;
use theme::Theme;

// TODO This information should come from our loaders
const SUPPORTED_AUDIO_FILES: [&str; 5] = ["aiff", "flac", "midi", "ogg", "wav"];

impl<T: AudioBackend> AudioController<T> {
    fn handle_pause(&mut self, response_sender: Sender<response::Generic>) {
        if self.theme_loaded {
            for (_, handle) in &mut self.sound_handles {
                if handle.is_in_state(&AudioEntityState::Playing) {
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
                if handle.is_in_state(&AudioEntityState::Playing) {
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
            handle.switch_state(AudioEntityState::Preview);
        }
    }

    fn handle_load_theme(&mut self, theme: Theme, response_sender: Sender<response::LoadTheme>) {
        self.sound_handles.clear();

        for sound in theme.sounds {
            let mut full_path: PathBuf = PathBuf::from(&self.sound_library);
            full_path.push(sound.file_path.clone());
            let object = self.backend.load_object(&full_path);

            info!("Loading file {} ...", &full_path.to_str().unwrap());

            self.sound_handles
                .insert(sound.name.clone(), AudioEntity::new(object, sound));
        }

        self.theme_loaded = true;

        response_sender.send(response::LoadTheme { success: true });

        info!("Theme loaded!")
    }

    fn handle_trigger(&mut self, sound: String, response_sender: Sender<response::Trigger>) {
        let mut success = false;
        if let Some(handle) = self.sound_handles.get_mut(&sound) {
            info!("Received trigger for sound '{}'!", sound);
            handle.is_triggered = !handle.is_triggered;
            success = true;
        } else {
            error!("Received trigger for unknown sound '{}'!", sound);
        }

        response_sender.send(response::Trigger {
            success: true,
            trigger_found: success,
        });
    }

    fn handle_get_status(&mut self, response_sender: Sender<response::Status>) {
        let mut playing: Vec<String> = Vec::new();
        for (name, handle) in &self.sound_handles {
            if handle.is_in_state(&AudioEntityState::Playing) {
                playing.push(name.to_string());
            }
        }

        response_sender.send(response::Status {
            playing: self.playing,
            theme_loaded: self.theme_loaded,
            sounds_playing: playing,
        });
    }

    fn handle_get_sound_library(&mut self, response_sender: Sender<response::SoundLibrary>) {
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

        response_sender.send(response::SoundLibrary { sounds: lib });
    }

    fn handle_volume(&mut self, value: f32) {
        self.backend.set_volume(value);
    }

    fn handle_get_driver_list(&mut self, response_sender: Sender<response::DriverList>) {
        let mut drivers: Vec<(i32, String)> = Vec::new();

        self.backend
            .get_output_devices()
            .into_iter()
            .for_each(|d| drivers.push((0, d)));

        response_sender.send(response::DriverList { drivers });
    }

    fn handle_get_driver(&mut self, response_sender: Sender<response::Driver>) {
        response_sender.send(response::Driver {
            id: self.backend.get_current_output_device(),
        });
    }

    fn handle_set_driver(&mut self, id: i32) {
        self.backend.set_current_output_device(id);
    }

    pub(in audio_engine::engine) fn run_message_queue(
        &mut self,
    ) -> Result<bool, AudioControllerError> {
        let timeout = Duration::from_millis(50);

        if let Ok(msg) = self.receiver.recv_timeout(timeout) {
            match msg {
                command::Command::Quit(data) => return Ok(true),
                command::Command::Pause(data) => self.handle_pause(data.response_sender.unwrap()),
                command::Command::Play(data) => self.handle_play(),
                command::Command::PreviewSound(data) => self.handle_preview_sound(data.sound),
                command::Command::LoadTheme(data) => {
                    self.handle_load_theme(data.theme, data.response_sender.unwrap())
                }
                command::Command::Trigger(data) => {
                    self.handle_trigger(data.sound, data.response_sender.unwrap())
                }

                command::Command::GetStatus(data) => {
                    self.handle_get_status(data.response_sender.unwrap())
                }

                command::Command::GetSoundLibrary(data) => {
                    self.handle_get_sound_library(data.response_sender.unwrap())
                }

                command::Command::Volume(data) => self.handle_volume(data.value),

                command::Command::GetDriverList(data) => {
                    self.handle_get_driver_list(data.response_sender.unwrap())
                }

                command::Command::GetDriver(data) => {
                    self.handle_get_driver(data.response_sender.unwrap())
                }

                command::Command::SetDriver(data) => self.handle_set_driver(data.id),
            }
        };

        Ok(false)
    }
}
