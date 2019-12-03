use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use audio_engine::backends::base::AudioBackend;
use audio_engine::engine::error::AudioEngineError;
use audio_engine::engine::AudioEntity;
use audio_engine::engine::{AudioController, AudioEntityState};
use audio_engine::messages::{command, response};
use theme::Theme;

// TODO This information should come from our loaders

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
        for (_, mut handle) in self.sound_handles.drain() {
            handle.stop(&mut self.backend)?;
        }

        for sound in theme.sounds {
            let sample_id = self
                .samplesdb
                .sample_id_by_path(&sound.file)
                .ok_or_else(|| AudioEngineError::SampleNotFound(sound.file.clone()))?;
            let full_path = self.samplesdb.full_path_of_sample(sample_id);

            let object = self.backend.load_file(&full_path).or_else(|e| {
                send_response!(self);
                Err(e)
            })?;

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
            send_error!(self, "Unknown sound '{}'!");
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
            build_response!(Status,
                playing: self.playing,
                theme_loaded: self.theme_loaded,
                theme: self.theme.clone(),
                sounds_playing: playing,
                sounds_playing_next: playing_next,
                previewing: previewing
            )
        );

        Ok(())
    }

    fn handle_get_sound_library(&mut self) -> Result<(), AudioEngineError> {
        let mut lib: Vec<String> = Vec::new();
        for entry in self.samplesdb.samples() {
            lib.push(entry.path.clone())
        }

        send_response!(self, build_response!(SoundLibrary, samples: lib));

        Ok(())
    }

    fn handle_volume(&mut self, value: f32) -> Result<(), AudioEngineError> {
        self.backend.set_volume(value);
        send_response!(self);

        Ok(())
    }

    fn handle_get_driver_list(&mut self) -> Result<(), AudioEngineError> {
        let mut drivers: Vec<(i32, String)> = Vec::new();

        self.backend
            .get_output_devices()
            .into_iter()
            .for_each(|d| drivers.push((0, d)));

        send_response!(self, build_response!(DriverList, drivers: drivers));

        Ok(())
    }

    fn handle_get_driver(&mut self) -> Result<(), AudioEngineError> {
        let id = self.backend.get_current_output_device();
        send_response!(self, build_response!(Driver, id: id));

        Ok(())
    }

    fn handle_set_driver(&mut self, id: i32) -> Result<(), AudioEngineError> {
        self.backend.set_current_output_device(id);

        send_response!(self);

        Ok(())
    }

    pub(in audio_engine::engine) fn run_message_queue(&mut self) -> Result<bool, AudioEngineError> {
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
