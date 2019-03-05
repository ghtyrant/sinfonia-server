#[macro_export]
macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::std::collections::HashMap::new();
         $( map.insert($key, $val); )*
         map
    }}
}

pub fn convert_to_mono(samples: Vec<i16>) -> Vec<i16> {
    let mut mono_samples = Vec::with_capacity(samples.len() / 2);
    for i in 0..samples.len() / 2 {
        mono_samples.push(((samples[i * 2] as i32 + samples[i * 2 + 1] as i32) / 2) as i16);
    }

    mono_samples
}
