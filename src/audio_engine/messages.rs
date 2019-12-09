use std::collections::HashMap;

use crate::theme::Theme;

#[derive(Serialize)]
pub enum Response {
    Error {
        message: String,
    },
    Success,
    Status {
        playing: bool,
        theme_loaded: bool,
        theme: Option<String>,
        sounds_playing: Vec<String>,
        sounds_playing_next: HashMap<String, u64>,
        previewing: Vec<String>,
    },

    LoadTheme {
        success: bool,
    },

    Trigger {
        success: bool,
        trigger_found: bool,
    },

    SoundLibrary {
        samples: Vec<(String, Vec<String>)>,
    },

    DriverList {
        drivers: HashMap<usize, String>,
    },

    Driver {
        id: i32,
    },
}

#[derive(Deserialize)]
pub enum Command {
    Quit,
    Play,
    Pause,
    GetStatus,
    GetSoundLibrary,
    GetDriver,
    GetDriverList,

    SetDriver { id: i32 },
    SetVolume { value: f32 },
    PreviewSound { sound: String },
    LoadTheme { theme: Theme },
    Trigger { sound: String },
}
