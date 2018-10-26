use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::fmt;
use std::marker::Send;
use std::time::Duration;

use serde_json;
use serde_json::Value;

use audio_engine::engine::SoundHandleParameters;
use error::ServerError;
use theme::FuncParameters;

pub trait SoundFunc: Send {
    fn execute(&mut self, params: &mut SoundHandleParameters);
    fn name(&self) -> &'static str;
    fn reset_state(&mut self);
}

impl fmt::Display for SoundFunc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

pub trait SoundFuncFactory: Send + Sync {
    fn new(&self, params: FuncParameters) -> Result<Box<SoundFunc>, ServerError>;
}

macro_rules! sound_func {
    ($main_struct: ident, $factory_struct: ident
    $param_struct: ident:
    { $($param_name: ident : $param_type: ty = $default: expr),* }
    $state_struct: ident:
    { $($state_param_name: ident : $state_param_type: ty = $state_default: expr),* }
    $body: expr) => {
        pub struct $state_struct {
            $(pub $state_param_name: $state_param_type),*
        }

        #[derive(Deserialize)]
        pub struct $param_struct {
            $(pub $param_name: [$param_type; 2]),*
        }

        impl $param_struct {
            $(pub fn $param_name(&self) -> $param_type {
                // rand::thread_rng() does not like the case when a == b, so we have to handle this here
                if self.$param_name[0] == self.$param_name[1] {
                    self.$param_name[0]
                } else {
                    thread_rng().gen_range(self.$param_name[0], self.$param_name[1])
                }
            })*
        }

        pub struct $main_struct {
            pub state: $state_struct,
            pub params: $param_struct,
        }

        impl $state_struct {
            fn reset(&mut self) {
                $(self.$state_param_name = $state_default);*
            }
        }

        impl Default for $state_struct {
            fn default() -> Self {
                Self {
                    $($state_param_name: $state_default),*
                }
            }
        }

        impl $main_struct {
            pub fn from_params(params: FuncParameters) -> Result<Self, serde_json::Error> {
                let mut map: HashMap<String, Value> = serde_json::from_value(params)?;

                $(if !map.contains_key(stringify!($param_name)) {
                    map.insert(stringify!($param_name).into(), serde_json::to_value($default)?);
                })*

                Ok(Self {
                    state: $state_struct::default(),
                    params: serde_json::from_value(serde_json::to_value(map)?)?
                })
            }
        }

        pub struct $factory_struct;

        impl SoundFunc for $main_struct {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            #[allow(unused_variables)]
            fn execute(&mut self, params: &mut SoundHandleParameters) {
                $body(&self.params, &mut self.state, params);
            }

            fn reset_state(&mut self) {
                self.state.reset();
            }
        }

        impl SoundFuncFactory for $factory_struct {
            fn new(&self, params: FuncParameters) -> Result<Box<SoundFunc>, ServerError>  {
                match $main_struct::from_params(params) {
                    Ok(func) => Ok(Box::new(func)),
                    Err(e) => { error!("Failed to parse SoundFunc parameters: {}", e); Err(ServerError::ParseFailed(e.to_string())) }
                }
            }
        }
    }
}

macro_rules! register_sound_funcs {
    ($($name: ident, $factory: ident),+) => (
        pub fn get_available_funcs() -> FuncMap {
            let mut map: FuncMap = HashMap::new();

            $(map.insert(stringify!($name).into(), Box::new($factory));)+

            map
        }
    )
}

pub type FuncMap = HashMap<String, Box<SoundFuncFactory>>;

sound_func!{
    Loop, LoopFactory

    LoopParams:
    {
        times: i32 = [0, 0]
    }

    LoopState:
    {
        played: i32 = 0
    }

    |params: &LoopParams, state: &mut LoopState, handle_params: &mut SoundHandleParameters|
    {
        info!("Loop with times: {}", params.times());

        if params.times() > 0 {
            if state.played < params.times() {
                state.played += 1;
            }
            else
            {
                state.played = 0;
                return;
            }
        }

        handle_params.should_loop = true;
    }
}

sound_func!{
    Repeat, RepeatFactory

    RepeatParams:
    {
        value: i32 = [1, 1]
    }

    RepeatState:
    {
    }

    |params: &RepeatParams, state: &mut RepeatState, handle_params: &mut SoundHandleParameters|
    {
        //handle_params.channel.as_ref().unwrap().set_mode(rfmod::types::Mode(rfmod::LOOP_NORMAL));
        //handle_params.channel.as_ref().unwrap().set_loop_count(params.value() - 1);
    }
}

sound_func!{
    Echo, EchoFactory

    EchoParams:
    {
        distance: i32 = [0, 0]
    }

    EchoState:
    {
    }

    |params: &EchoParams, state: &mut EchoState, handle_params: &mut SoundHandleParameters|
    {
        //handle_params.dsps["echo"].set_bypass(false);
        //handle_params.dsps["echo"].set_parameter(rfmod::DspTypeEcho::Delay as i32, params.distance() as f32);
    }
}

sound_func!{
    LowPass, LowPassFactory

    LowPassParams:
    {
        cutoff: i32 = [5000, 5000],
        resonance: f32 = [1.0, 1.0]
    }

    LowPassState:
    {
    }

    |params: &LowPassParams, state: &mut LowPassState, handle_params: &mut SoundHandleParameters|
    {
        //handle_params.dsps["lowpass"].set_bypass(false);
        //handle_params.dsps["lowpass"].set_parameter(rfmod::DspLowPass::Cutoff as i32, params.cutoff() as f32);
        //handle_params.dsps["lowpass"].set_parameter(rfmod::DspLowPass::Resonance as i32, params.resonance());
    }
}

sound_func!{
    Reverb, ReverbFactory

    ReverbParams:
    {
        dry_level: f32 = [0.0, 0.0],
        room: f32 = [-10000.0, -10000.0],
        room_lf: f32 = [0.0, 0.0],
        room_hf: f32 = [0.0, 0.0],
        decay_time: f32 = [1.0, 1.0],
        decay_hf_ratio: f32 = [0.5, 0.5],
        reflections_level: f32 = [-10000.0, -10000.0],
        reflections_delay: f32 = [0.02, 0.02],
        reverb_level: f32 = [0.0, 0.0],
        reverb_delay: f32 = [0.04, 0.04],
        diffusion: f32 = [100.0, 100.0],
        density: f32 = [100.0, 100.0],
        hf_reference: f32 = [5000.0, 5000.0],
        lf_reference: f32 = [250.0, 250.0]
    }

    ReverbState:
    {
    }

    |params: &ReverbParams, state: &mut ReverbState, handle_params: &mut SoundHandleParameters|
    {
        /*handle_params.dsps["reverb"].set_bypass(false);
        handle_params.dsps["reverb"].set_parameter(rfmod::DspSfxReverb::DryLevel as i32, params.dry_level());
        handle_params.dsps["reverb"].set_parameter(rfmod::DspSfxReverb::Room as i32, params.room());
        handle_params.dsps["reverb"].set_parameter(rfmod::DspSfxReverb::DecayTime as i32, params.decay_time());
        handle_params.dsps["reverb"].set_parameter(rfmod::DspSfxReverb::DecayHFRatio as i32, params.decay_hf_ratio());
        handle_params.dsps["reverb"].set_parameter(rfmod::DspSfxReverb::ReflectionsLevel as i32, params.reflections_level());
        handle_params.dsps["reverb"].set_parameter(rfmod::DspSfxReverb::ReflectionsDelay as i32, params.reflections_delay());
        handle_params.dsps["reverb"].set_parameter(rfmod::DspSfxReverb::ReverbLevel as i32, params.reverb_level());
        handle_params.dsps["reverb"].set_parameter(rfmod::DspSfxReverb::ReverbDelay as i32, params.reverb_delay());
        handle_params.dsps["reverb"].set_parameter(rfmod::DspSfxReverb::Diffusion as i32, params.diffusion());
        handle_params.dsps["reverb"].set_parameter(rfmod::DspSfxReverb::Density as i32, params.density());
        handle_params.dsps["reverb"].set_parameter(rfmod::DspSfxReverb::HFReference as i32, params.hf_reference());
        handle_params.dsps["reverb"].set_parameter(rfmod::DspSfxReverb::RoomLF as i32, params.room_lf());
        handle_params.dsps["reverb"].set_parameter(rfmod::DspSfxReverb::RoomHF as i32, params.room_hf());
        handle_params.dsps["reverb"].set_parameter(rfmod::DspSfxReverb::LFReference as i32, params.lf_reference());*/
    }
}

sound_func!{
    Delay, DelayFactory

    DelayParams:
    {
        value: u64 = [0, 0]
    }

    DelayState:
    {
    }

    |params: &DelayParams, state: &mut DelayState, handle_params: &mut SoundHandleParameters|
    {
        handle_params.next_play = Duration::from_millis(params.value());
    }
}

sound_func!{
    Volume, VolumeFactory

    VolumeParams:
    {
        value: f32 = [1.0, 1.0]
    }

    VolumeState:
    {
    }

    |params: &VolumeParams, state: &mut VolumeState, handle_params: &mut SoundHandleParameters|
    {
        //handle_params.channel.as_ref().unwrap().set_volume(params.value());
    }
}

sound_func!{
    Frequency, FrequencyFactory

    FrequencyParams:
    {
        value: f32 = [1.0, 1.0]
    }

    FrequencyState:
    {
    }

    |params: &FrequencyParams, state: &mut FrequencyState, handle_params: &mut SoundHandleParameters|
    {
        /*let freq = match handle_params.channel.as_ref().unwrap().get_frequency() {
            Ok(freq) => freq,
            Err(e) => { error!("Error getting frequency of channel: {:?}", e); return }
        };
        handle_params.channel.as_ref().unwrap().set_frequency(freq * params.value());*/
    }
}

sound_func!{
    Fader, FaderFactory

    FaderParams:
    {
        fade_in_length: f32 = [0.0, 0.0],
        fade_out_length: f32 = [0.0, 0.0]
    }

    FaderState:
    {
        max_volume_set: bool = false,
        max_volume: f32 = 0.0
    }

    |params: &FaderParams, state: &mut FaderState, handle_params: &mut SoundHandleParameters|
    {
        /*if !state.max_volume_set {
            state.max_volume = match handle_params.channel.as_ref().unwrap().get_volume() {
                Ok(vol) => vol,
                Err(_) => { return }
            };

            state.max_volume_set = true;
        }

        let sound = match handle_params.channel.as_ref().unwrap().get_current_sound() {
            Ok(sound) => sound,
            Err(_) => { return }
        };

        match sound.get_length(TimeUnit(1)) {
            Ok(len) => {

                let position = match handle_params.channel.as_ref().unwrap().get_position(TimeUnit(1)) {
                    Ok(pos) => pos,
                    Err(_) => return
                } as f32;

                let fade_in_length = params.fade_in_length();
                if fade_in_length > 0.0 {
                    let fade_in_len = len as f32 * fade_in_length;

                    if position < fade_in_len {
                        handle_params.channel
                            .as_ref()
                            .unwrap()
                            .set_volume((1.0 - (fade_in_len - position) / fade_in_len) * state.max_volume);
                    }
                }

                let fade_out_length = params.fade_out_length();
                if fade_out_length > 0.0 {
                    let fade_out_len = len as f32 * fade_out_length;
                    let fade_out_start = len as f32 - fade_out_len;

                    if position > fade_out_start {
                        handle_params.channel
                            .as_ref()
                            .unwrap()
                            .set_volume((1.0 - (position - fade_out_start) / fade_out_len) * state.max_volume);
                    }
                }
            },
            Err(_) => { error!("Could not get sound length!"); }
        }*/
    }
}

register_sound_funcs! {
    Loop, LoopFactory,
    Repeat, RepeatFactory,
    Delay, DelayFactory,
    Volume, VolumeFactory,
    Frequency, FrequencyFactory,
    Fader, FaderFactory,
    Echo, EchoFactory,
    LowPass, LowPassFactory,
    Reverb, ReverbFactory
}
