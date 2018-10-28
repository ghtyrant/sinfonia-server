use std::path::PathBuf;
use std::time::Duration;

use audio_engine::backends::base::AudioBackend;
use audio_engine::engine::AudioEntity;
use audio_engine::engine::{AudioController, AudioEntityState};
#[macro_use]
use audio_engine::messages;
use audio_engine::messages::{command, response};
use error::AudioEngineError;
use theme::Theme;

// TODO This information should come from our loaders
const SUPPORTED_AUDIO_FILES: [&str; 5] = ["aiff", "flac", "midi", "ogg", "wav"];

/*
self.sender
    .send(response::Response::Trigger(response::Trigger {
        success: true,
        trigger_found: success,
    }));
*/

macro_rules! send_response {
    ($self: ident) => {
        $self
            .sender
            .send(build_response!(Success))
            .expect("Failed to communicate with API!");
    };

    ($self: ident, $message: expr) => {
        $self
            .sender
            .send($message)
            .expect("Failed to communicate with API!");
    };
}

macro_rules! send_error {
    ($self: ident, $message: expr) => {
        $self
            .sender
            .send(build_response!(Error, message: $message.to_string()))
            .expect("Failed to communicate with API!");
    };
}

impl<T: AudioBackend> AudioController<T> {
    fn handle_pause(&mut self) -> Result<(), AudioEngineError> {
        if self.theme_loaded {
            for handle in &mut self.sound_handles.values_mut() {
                if handle.is_in_state(&AudioEntityState::Playing) {
                    handle.pause(true);
                }
            }

            self.playing = false;
            send_response!(self);
            info!("Paused!");
        } else {
            debug!("No theme loaded, not pausing ...");
            send_error!(self, "No theme loaded!");
        }

        Ok(())
    }

    fn handle_play(&mut self) -> Result<(), AudioEngineError> {
        if self.theme_loaded {
            for (_, handle) in &mut self.sound_handles {
                if handle.is_in_state(&AudioEntityState::Playing) {
                    handle.pause(false);
                }
            }

            self.playing = true;

            info!("Playing!");
        } else {
            debug!("No theme loaded, not playing ...");
        }

        Ok(())
    }

    fn handle_preview_sound(&mut self, sound: String) -> Result<(), AudioEngineError> {
        if let Some(handle) = self.sound_handles.get_mut(&sound) {
            handle.is_preview = true;
            handle.switch_state(AudioEntityState::Preview);
        }

        Ok(())
    }

    fn handle_load_theme(&mut self, theme: Theme) -> Result<(), AudioEngineError> {
        self.sound_handles.clear();

        for sound in theme.sounds {
            let mut full_path: PathBuf = PathBuf::from(&self.sound_library);
            full_path.push(sound.file_path.clone());
            let object = self.backend.load_file(&full_path)?;

            info!("Loading file {} ...", &full_path.to_str().unwrap());

            self.sound_handles
                .insert(sound.name.clone(), AudioEntity::new(object, sound));
        }

        self.theme_loaded = true;

        self.sender
            .send(response::Response::LoadTheme(response::LoadTheme {
                success: true,
            }));

        info!("Theme loaded!");

        Ok(())
    }

    fn handle_trigger(&mut self, sound: String) -> Result<(), AudioEngineError> {
        let mut success = false;
        if let Some(handle) = self.sound_handles.get_mut(&sound) {
            info!("Received trigger for sound '{}'!", sound);
            handle.is_triggered = !handle.is_triggered;
            success = true;
        } else {
            error!("Received trigger for unknown sound '{}'!", sound);
        }

        self.sender
            .send(response::Response::Trigger(response::Trigger {
                success: true,
                trigger_found: success,
            }));

        Ok(())
    }

    fn handle_get_status(&mut self) -> Result<(), AudioEngineError> {
        let mut playing: Vec<String> = Vec::new();
        for (name, handle) in &self.sound_handles {
            if handle.is_in_state(&AudioEntityState::Playing) {
                playing.push(name.to_string());
            }
        }

        self.sender
            .send(response::Response::Status(response::Status {
                playing: self.playing,
                theme_loaded: self.theme_loaded,
                sounds_playing: playing,
            }));

        Ok(())
    }

    fn handle_get_sound_library(&mut self) -> Result<(), AudioEngineError> {
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

        self.sender
            .send(response::Response::SoundLibrary(response::SoundLibrary {
                sounds: lib,
            }));

        Ok(())
    }

    fn handle_volume(&mut self, value: f32) -> Result<(), AudioEngineError> {
        self.backend.set_volume(value);

        Ok(())
    }

    fn handle_get_driver_list(&mut self) -> Result<(), AudioEngineError> {
        let mut drivers: Vec<(i32, String)> = Vec::new();

        self.backend
            .get_output_devices()
            .into_iter()
            .for_each(|d| drivers.push((0, d)));

        self.sender
            .send(response::Response::DriverList(response::DriverList {
                drivers,
            }));

        Ok(())
    }

    fn handle_get_driver(&mut self) -> Result<(), AudioEngineError> {
        self.sender
            .send(response::Response::Driver(response::Driver {
                id: self.backend.get_current_output_device(),
            }));

        Ok(())
    }

    fn handle_set_driver(&mut self, id: i32) -> Result<(), AudioEngineError> {
        self.backend.set_current_output_device(id);

        Ok(())
    }

    pub(in audio_engine::engine) fn run_message_queue(&mut self) -> Result<bool, AudioEngineError> {
        let timeout = Duration::from_millis(50);

        if let Ok(msg) = self.receiver.recv_timeout(timeout) {
            match msg {
                command::Command::Quit(data) => return Ok(true),
                command::Command::Pause(data) => self.handle_pause()?,
                command::Command::Play(data) => self.handle_play()?,
                command::Command::PreviewSound(data) => self.handle_preview_sound(data.sound)?,
                command::Command::LoadTheme(data) => self.handle_load_theme(data.theme)?,
                command::Command::Trigger(data) => self.handle_trigger(data.sound)?,

                command::Command::GetStatus(data) => self.handle_get_status()?,

                command::Command::GetSoundLibrary(data) => self.handle_get_sound_library()?,

                command::Command::Volume(data) => self.handle_volume(data.value)?,

                command::Command::GetDriverList(data) => self.handle_get_driver_list()?,

                command::Command::GetDriver(data) => self.handle_get_driver()?,

                command::Command::SetDriver(data) => self.handle_set_driver(data.id)?,
            }
        };

        Ok(false)
    }
}
