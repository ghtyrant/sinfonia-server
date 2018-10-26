use std::sync::mpsc::{Receiver, Sender};

use theme::Theme;

#[derive(Serialize)]
pub struct AudioControllerStatus {
    pub playing: bool,
    pub theme_loaded: bool,
    pub sounds_playing: Vec<String>,
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