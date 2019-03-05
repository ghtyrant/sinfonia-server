use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use audio_engine::backends::base::AudioBackend;
use audio_engine::engine::AudioEntity;
use audio_engine::engine::{AudioController, AudioEntityState};
use audio_engine::messages::{command, response};
use error::SinfoniaGenericError;
use theme::Theme;

// TODO This information should come from our loaders
const SUPPORTED_AUDIO_FILES: [&str; 6] = ["aiff", "flac", "midi", "ogg", "wav", "mp3"];

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
    fn handle_pause(&mut self) -> Result<(), SinfoniaGenericError> {
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

    fn handle_play(&mut self) -> Result<(), SinfoniaGenericError> {
        if self.theme_loaded {
            for (_, handle) in &mut self.sound_handles {
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

    fn handle_preview_sound(&mut self, sound: String) -> Result<(), SinfoniaGenericError> {
        if let Some(handle) = self.sound_handles.get_mut(&sound) {
            handle.is_preview = true;
            handle.switch_state(AudioEntityState::Preview);

            send_response!(self);
            info!("Playing!");
        } else {
            debug!("handle_preview_sound(): No such sound {}", sound);
            send_error!(self, "No such sound {}");
        }

        Ok(())
    }

    fn handle_load_theme(&mut self, theme: Theme) -> Result<(), SinfoniaGenericError> {
        for (_, mut handle) in self.sound_handles.drain() {
            handle.stop(&mut self.backend);
        }

        for sound in theme.sounds {
            let mut full_path: PathBuf = PathBuf::from(&self.sound_library);
            full_path.push(sound.file.clone());
            let object = self.backend.load_file(&full_path)?;

            info!("Loading file {} ...", &full_path.to_str().unwrap());

            self.sound_handles.insert(
                sound.name.clone(),
                AudioEntity::<T::EntityData>::new(object, sound),
            );
        }

        self.theme = Some(theme.name);
        self.theme_loaded = true;

        send_response!(self);

        info!("Theme loaded!");

        Ok(())
    }

    fn handle_trigger(&mut self, sound: String) -> Result<(), SinfoniaGenericError> {
        if let Some(handle) = self.sound_handles.get_mut(&sound) {
            info!("handle_trigger(): Received trigger for sound '{}'!", sound);
            handle.is_triggered = !handle.is_triggered;

            send_response!(self);
        } else {
            error!(
                "handle_trigger(): Received trigger for unknown sound '{}'!",
                sound
            );
            send_error!(self, "Unknown sound '{}'!");
        }

        Ok(())
    }

    fn handle_get_status(&mut self) -> Result<(), SinfoniaGenericError> {
        let mut playing: Vec<String> = Vec::new();
        let mut playing_next: HashMap<String, u64> = HashMap::new();

        for (name, handle) in &self.sound_handles {
            if handle.is_in_state(&AudioEntityState::Playing) {
                playing.push(name.to_string());
            } else if handle.is_in_state(&AudioEntityState::WaitingForStart) {
                playing_next.insert(name.to_string(), handle.parameters.next_play.as_secs());
            }
        }

        send_response!(
            self,
            build_response!(Status,
                playing: self.playing,
                theme_loaded: self.theme_loaded,
                theme: self.theme.clone(),
                sounds_playing: playing,
                sounds_playing_next: playing_next
            )
        );

        Ok(())
    }

    fn handle_get_sound_library(&mut self) -> Result<(), SinfoniaGenericError> {
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

        send_response!(self, build_response!(SoundLibrary, samples: lib));

        Ok(())
    }

    fn handle_volume(&mut self, value: f32) -> Result<(), SinfoniaGenericError> {
        self.backend.set_volume(value);
        send_response!(self);

        Ok(())
    }

    fn handle_get_driver_list(&mut self) -> Result<(), SinfoniaGenericError> {
        let mut drivers: Vec<(i32, String)> = Vec::new();

        self.backend
            .get_output_devices()
            .into_iter()
            .for_each(|d| drivers.push((0, d)));

        send_response!(self, build_response!(DriverList, drivers: drivers));

        Ok(())
    }

    fn handle_get_driver(&mut self) -> Result<(), SinfoniaGenericError> {
        let id = self.backend.get_current_output_device();
        send_response!(self, build_response!(Driver, id: id));

        Ok(())
    }

    fn handle_set_driver(&mut self, id: i32) -> Result<(), SinfoniaGenericError> {
        self.backend.set_current_output_device(id);

        send_response!(self);

        Ok(())
    }

    pub(in audio_engine::engine) fn run_message_queue(
        &mut self,
    ) -> Result<bool, SinfoniaGenericError> {
        let timeout = Duration::from_millis(50);

        if let Ok(msg) = self.receiver.recv_timeout(timeout) {
            match msg {
                command::Command::Quit(_) => return Ok(true),
                command::Command::Pause(_) => self.handle_pause()?,
                command::Command::Play(_) => self.handle_play()?,
                command::Command::PreviewSound(data) => self.handle_preview_sound(data.sound)?,
                command::Command::LoadTheme(data) => self.handle_load_theme(data.theme)?,
                command::Command::Trigger(data) => self.handle_trigger(data.sound)?,
                command::Command::GetStatus(_) => self.handle_get_status()?,
                command::Command::GetSoundLibrary(_) => self.handle_get_sound_library()?,
                command::Command::Volume(data) => self.handle_volume(data.value)?,
                command::Command::GetDriverList(_) => self.handle_get_driver_list()?,
                command::Command::GetDriver(_) => self.handle_get_driver()?,
                command::Command::SetDriver(data) => self.handle_set_driver(data.id)?,
            }
        };

        Ok(false)
    }
}
