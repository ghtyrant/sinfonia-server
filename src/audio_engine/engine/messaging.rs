use std::collections::HashMap;
use std::time::Duration;

use crate::audio_engine::backends::base::AudioBackend;
use crate::audio_engine::engine::error::AudioEngineError;
use crate::audio_engine::engine::AudioEntity;
use crate::audio_engine::engine::{AudioController, AudioEntityState};
use crate::audio_engine::messages::{Command, Response};
use crate::theme::Theme;

// TODO This information should come from our loaders

macro_rules! send_response {
    ($self: ident) => {
        $self
            .sender
            .send(Response::Success)
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
            .send(Response::Error {
                message: $message.to_string(),
            })
            .expect("Failed to communicate with API!");
    };
}

impl<'a, T: AudioBackend> AudioController<'a, T> {
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
            for handle in &mut self.sound_handles.values_mut() {
                if handle.is_in_state(&AudioEntityState::Playing) {
                    handle.pause(false);
                }
            }

            self.playing = true;

            send_response!(self);
            info!("Playing!");
        } else {
            debug!("No theme loaded, not playing ...");
            send_error!(self, "No theme loaded!");
        }

        Ok(())
    }

    fn handle_preview_sound(&mut self, sound: String) -> Result<(), AudioEngineError> {
        if let Some(handle) = self.sound_handles.get_mut(&sound) {
            handle.is_preview = true;
            handle.switch_state(AudioEntityState::Preview);
            info!("Starting preview of sound '{}'!", sound);

            send_response!(self);
        } else {
            debug!("handle_preview_sound(): No such sound {}", sound);
            send_error!(self, "No such sound {}");
        }

        Ok(())
    }

    fn handle_load_theme(&mut self, theme: Theme) -> Result<(), AudioEngineError> {
        let mut handles = HashMap::new();
        for sound in theme.sounds {
            let sample_id = match self.samplesdb.sample_id_by_path(&sound.file) {
                Some(id) => id,
                None => {
                    send_error!(self, format!("No such sound {}", sound.file));
                    return Err(AudioEngineError::SampleNotFound(sound.file.clone()));
                }
            };
            let full_path = self.samplesdb.full_path_of_sample(sample_id);

            info!("Loading file {} ...", &full_path.to_str().unwrap());

            let object = self.backend.load_file(&full_path).or_else(|e| {
                send_response!(self);
                Err(e)
            })?;

            handles.insert(
                sound.name.clone(),
                AudioEntity::<T::EntityData>::new(object, sound),
            );
        }

        self.next_sound_handles = Some(handles);

        self.theme = Some(theme.name);
        self.theme_loaded = true;

        send_response!(self);

        info!("Theme loaded!");

        Ok(())
    }

    fn handle_trigger(&mut self, sound: String) -> Result<(), AudioEngineError> {
        if let Some(handle) = self.sound_handles.get_mut(&sound) {
            info!("handle_trigger(): Received trigger for sound '{}'!", sound);
            handle.is_triggered = !handle.is_triggered;

            send_response!(self);
        } else {
            error!(
                "handle_trigger(): Received trigger for unknown sound '{}'!",
                sound
            );
            send_error!(self, format!("Unknown sound '{}'!", sound));
        }

        Ok(())
    }

    fn handle_get_status(&mut self) -> Result<(), AudioEngineError> {
        let mut playing: Vec<String> = Vec::new();
        let mut playing_next: HashMap<String, u64> = HashMap::new();
        let mut previewing: Vec<String> = Vec::new();

        for (name, handle) in &self.sound_handles {
            if handle.is_in_state(&AudioEntityState::Playing) {
                playing.push(name.to_string());
            } else if handle.is_in_state(&AudioEntityState::WaitingForStart) {
                playing_next.insert(name.to_string(), handle.parameters.next_play.as_secs());
            }

            if handle.is_preview {
                previewing.push(name.to_string());
            }
        }

        send_response!(
            self,
            Response::Status {
                playing: self.playing,
                theme_loaded: self.theme_loaded,
                theme: self.theme.clone(),
                sounds_playing: playing,
                sounds_playing_next: playing_next,
                previewing: previewing
            }
        );

        Ok(())
    }

    fn handle_get_sound_library(&mut self) -> Result<(), AudioEngineError> {
        let mut lib: Vec<String> = Vec::new();
        for entry in self.samplesdb.samples() {
            lib.push(entry.path.clone())
        }

        let samples = self
            .samplesdb
            .samples()
            .map(|sample| {
                (
                    sample.path.clone(),
                    sample.tags.iter().map(|&tag| tag.name.clone()).collect(),
                )
            })
            .collect();

        send_response!(self, Response::SoundLibrary { samples });

        Ok(())
    }

    fn handle_volume(&mut self, value: f32) -> Result<(), AudioEngineError> {
        self.backend.set_volume(value);
        self.master_volume = value;
        send_response!(self);

        Ok(())
    }

    fn handle_get_driver_list(&mut self) -> Result<(), AudioEngineError> {
        let drivers = self
            .backend
            .get_output_devices()
            .into_iter()
            .enumerate()
            .collect();

        send_response!(self, Response::DriverList { drivers });

        Ok(())
    }

    fn handle_get_driver(&mut self) -> Result<(), AudioEngineError> {
        let id = self.backend.get_current_output_device();
        send_response!(self, Response::Driver { id });

        Ok(())
    }

    fn handle_set_driver(&mut self, id: i32) -> Result<(), AudioEngineError> {
        self.backend.set_current_output_device(id);

        send_response!(self);

        Ok(())
    }

    pub(in crate::audio_engine::engine) fn run_message_queue(
        &mut self,
    ) -> Result<bool, AudioEngineError> {
        let timeout = Duration::from_millis(50);

        if let Ok(msg) = self.receiver.recv_timeout(timeout) {
            match msg {
                Command::Quit => return Ok(true),
                Command::Pause => self.handle_pause()?,
                Command::Play => self.handle_play()?,
                Command::PreviewSound { sound } => self.handle_preview_sound(sound)?,
                Command::LoadTheme { theme } => self.handle_load_theme(theme)?,
                Command::Trigger { sound } => self.handle_trigger(sound)?,
                Command::GetStatus => self.handle_get_status()?,
                Command::GetSoundLibrary => self.handle_get_sound_library()?,
                Command::SetVolume { value } => self.handle_volume(value)?,
                Command::GetDriverList => self.handle_get_driver_list()?,
                Command::GetDriver => self.handle_get_driver()?,
                Command::SetDriver { id } => self.handle_set_driver(id)?,
            }
        };

        Ok(false)
    }
}
