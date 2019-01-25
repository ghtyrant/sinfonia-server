use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use serde_json::Value;

fn get_default_count() -> (u32, u32) {
    (1, 1)
}

fn get_default_delay() -> (u64, u64) {
    (0, 0)
}

#[derive(Deserialize)]
pub struct Sound {
    pub name: String,
    pub file: String,
    pub volume: f32,
    pub trigger: Option<String>,
    pub enabled: bool,

    #[serde(default = "get_default_count")]
    pub repeat_count: (u32, u32),

    #[serde(default = "get_default_delay")]
    pub repeat_delay: (u64, u64),

    #[serde(default = "get_default_count")]
    pub loop_count: (u32, u32),

    #[serde(default = "get_default_delay")]
    pub loop_delay: (u64, u64),

    #[serde(default)]
    pub loops_forever: bool,
}

/*
impl Sound {
    pub fn new(
        name: String,
        file: String,
        volume: f32,
        trigger: Option<String>,
        enabled: bool,
        loop_count: (i32, i32),
    ) -> Self {
        Self {
            name,
            file,
            volume,
            trigger,
            enabled,
            loop_count,
        }
    }
}

impl<'de> Deserialize<'de> for Sound {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Name,
            File,
            Volume,
            Trigger,
            Enabled,
            Category,
            LoopCount,
        }

        struct SoundVisitor;

        impl<'de> Visitor<'de> for SoundVisitor {
            type Value = Sound;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Sound")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Sound, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut name = None;
                let mut file = None;
                let mut volume = None;
                let mut trigger = None;
                let mut enabled = None;
                let mut loop_count = None;

                while let Some(key) = map.next_key()? {
                    debug!("Parsing sound field '{:?}' ...", key);

                    match key {
                        Field::Name => {
                            if name.is_some() {
                                return Err(de::Error::duplicate_field("name"));
                            }

                            name = Some(map.next_value()?);
                        }

                        Field::File => {
                            if file.is_some() {
                                return Err(de::Error::duplicate_field("file"));
                            }

                            file = Some(map.next_value()?);
                        }

                        Field::Volume => {
                            if volume.is_some() {
                                return Err(de::Error::duplicate_field("volume"));
                            }

                            volume = Some(map.next_value()?);
                        }

                        Field::Trigger => {
                            if trigger.is_some() {
                                return Err(de::Error::duplicate_field("trigger"));
                            }

                            trigger = Some(map.next_value()?);
                        }

                        Field::Enabled => {
                            if enabled.is_some() {
                                return Err(de::Error::duplicate_field("enabled"));
                            }

                            enabled = Some(map.next_value()?);
                        }

                        Field::LoopCount => {
                            if loop_count.is_some() {
                                return Err(de::Error::duplicate_field("loop_count"));
                            }

                            loop_count = Some(map.next_value()?);
                        }

                        _ => {
                            info!("Unknown field '{:?}', skipping ...", key);
                            map.next_value::<Value>()?;
                        }
                    }
                }

                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                let file = file.ok_or_else(|| de::Error::missing_field("file"))?;

                let enabled = match enabled {
                    Some(flag) => flag,
                    None => false,
                };

                let volume = match volume {
                    Some(value) => value,
                    None => 1.0,
                };

                let loop_count = match loop_count {
                    Some((v1, v2)) => (v1, v2),
                    None => (0, 0),
                };

                Ok(Sound::new(name, file, volume, trigger, enabled, loop_count))
            }
        }

        const FIELDS: &[&str] = &["name", "file", "volume", "trigger", "enabled", "loop_count"];
        deserializer.deserialize_struct("Sound", FIELDS, SoundVisitor)
    }
}*/

#[derive(Deserialize)]
pub struct Theme {
    pub name: String,
    pub sounds: Vec<Sound>,
}
