use std::collections::HashMap;
use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use serde_json::Value;

use sound_funcs::{get_available_funcs, FuncMap, SoundFunc};

pub type FuncParameters = Value;
pub type FuncList = Vec<Box<SoundFunc>>;

pub const FUNC_TYPE_START: usize = 0;
pub const FUNC_TYPE_UPDATE: usize = 1;
pub const FUNC_TYPE_FINISH: usize = 2;

pub struct Sound {
    pub name: String,
    pub file_path: String,
    pub funcs: [FuncList; 3],
    pub needs_trigger: bool,
    pub is_disabled: bool,
}

impl Sound {
    pub fn new(
        name: String,
        file_path: String,
        funcs: [FuncList; 3],
        needs_trigger: bool,
        is_disabled: bool,
    ) -> Self {
        Self {
            name,
            file_path,
            funcs,
            needs_trigger,
            is_disabled,
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
            FilePath,
            Funcs,
            NeedsTrigger,
            IsDisabled,
            Category,
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
                let mut file_path = None;
                let mut funcs: [FuncList; 3] = [Vec::new(), Vec::new(), Vec::new()];
                let mut needs_trigger = None;
                let mut is_disabled = None;

                let available_funcs = get_available_funcs();

                fn parse_funcs<'de, V>(
                    available_funcs: &FuncMap,
                    func_map: &mut HashMap<String, HashMap<String, Value>>,
                ) -> Result<[FuncList; 3], V::Error>
                where
                    V: MapAccess<'de>,
                {
                    let mut functions: [FuncList; 3] = [Vec::new(), Vec::new(), Vec::new()];

                    for (func_type, mut funcs) in func_map.drain() {
                        let index = match func_type.as_ref() {
                            "start" => 0,
                            "update" => 1,
                            "finish" => 2,
                            _ => {
                                return Err(de::Error::custom(format!(
                                    "unkown func type '{}'",
                                    &func_type
                                )));
                            }
                        };

                        for (func_name, params) in funcs.drain() {
                            if !available_funcs.contains_key(&func_name) {
                                return Err(de::Error::custom(format!(
                                    "unkown function '{}'",
                                    &func_name
                                )));
                            }

                            let func = match available_funcs[&func_name].new(params) {
                                Ok(func) => func,
                                Err(e) => {
                                    return Err(de::Error::custom(e.to_string()));
                                }
                            };

                            functions[index].push(func);
                        }
                    }

                    Ok(functions)
                }

                while let Some(key) = map.next_key()? {
                    debug!("Parsing sound field '{:?}' ...", key);

                    match key {
                        Field::Name => {
                            if name.is_some() {
                                return Err(de::Error::duplicate_field("name"));
                            }

                            name = Some(map.next_value()?);
                        }

                        Field::FilePath => {
                            if file_path.is_some() {
                                return Err(de::Error::duplicate_field("file_path"));
                            }

                            file_path = Some(map.next_value()?);
                        }

                        Field::Funcs => {
                            if !funcs[FUNC_TYPE_START].is_empty()
                                || !funcs[FUNC_TYPE_UPDATE].is_empty()
                                || !funcs[FUNC_TYPE_FINISH].is_empty()
                            {
                                return Err(de::Error::duplicate_field("funcs"));
                            }

                            funcs = parse_funcs::<V>(&available_funcs, &mut map.next_value()?)?
                        }

                        Field::NeedsTrigger => {
                            if needs_trigger.is_some() {
                                return Err(de::Error::duplicate_field("needs_trigger"));
                            }

                            needs_trigger = Some(map.next_value()?);
                        }

                        Field::IsDisabled => {
                            if is_disabled.is_some() {
                                return Err(de::Error::duplicate_field("is_disabled"));
                            }

                            is_disabled = Some(map.next_value()?);
                        }

                        _ => {
                            info!("Unknown field '{:?}', skipping ...", key);
                            map.next_value::<Value>()?;
                        }
                    }
                }

                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                let file_path = file_path.ok_or_else(|| de::Error::missing_field("file_path"))?;

                let needs_trigger = match needs_trigger {
                    Some(flag) => flag,
                    None => false,
                };

                let is_disabled = match is_disabled {
                    Some(flag) => flag,
                    None => false,
                };

                Ok(Sound::new(
                    name,
                    file_path,
                    funcs,
                    needs_trigger,
                    is_disabled,
                ))
            }
        }

        const FIELDS: &[&str] = &[
            "name",
            "file_path",
            "finish_funcs",
            "update_funcs",
            "start_funcs",
            "needs_trigger",
            "is_disabled",
        ];
        deserializer.deserialize_struct("Sound", FIELDS, SoundVisitor)
    }
}

#[derive(Deserialize)]
pub struct Theme {
    pub name: String,
    pub sounds: Vec<Sound>,
}
