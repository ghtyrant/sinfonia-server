fn get_default_count() -> (u32, u32) {
    (0, 0)
}

fn get_default_pitch() -> (f32, f32) {
    (1.0, 1.0)
}

fn get_default_delay() -> (u64, u64) {
    (0, 0)
}

fn get_default_reverb() -> String {
    "none".to_string()
}

#[derive(Deserialize)]
pub struct Sound {
    pub name: String,
    pub file: String,
    pub volume: (f32, f32),
    pub trigger: Option<String>,
    pub enabled: bool,

    #[serde(default = "get_default_reverb")]
    pub reverb: String,

    #[serde(default = "get_default_count")]
    pub repeat_count: (u32, u32),

    #[serde(default = "get_default_delay")]
    pub repeat_delay: (u64, u64),

    #[serde(default = "get_default_count")]
    pub loop_count: (u32, u32),

    #[serde(default = "get_default_delay")]
    pub loop_delay: (u64, u64),

    #[serde(default)]
    pub loop_forever: bool,

    #[serde(default)]
    pub pitch_enabled: bool,

    #[serde(default = "get_default_pitch")]
    pub pitch: (f32, f32),

    #[serde(default)]
    pub lowpass_enabled: bool,

    #[serde(default = "get_default_pitch")]
    pub lowpass: (f32, f32),
}

#[derive(Deserialize)]
pub struct Theme {
    pub name: String,
    pub room: String,
    pub sounds: Vec<Sound>,
}
