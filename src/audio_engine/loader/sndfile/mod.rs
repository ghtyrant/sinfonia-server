mod sndfile_wrapper;

use itertools::Itertools;
use num::{Integer, NumCast, PrimInt};
use std::path::PathBuf;

use audio_engine::loader::base::AudioFileLoader;
use audio_engine::loader::sndfile::sndfile_wrapper::{OpenMode, SndFile};

pub struct SndFileLoader;

fn convert_to_mono<N>(samples: Vec<N>) -> Vec<N>
where
    N: Integer + PrimInt + std::iter::Sum,
{
    samples
        .into_iter()
        .chunks(2)
        .into_iter()
        .map::<N, _>(|a| (a.sum::<N>() / NumCast::from(2).unwrap()))
        .collect()
}

impl AudioFileLoader for SndFileLoader {
    fn load(&mut self, path: &PathBuf) -> (Vec<i16>, i32) {
        let mut s = SndFile::new(&path.to_str().unwrap(), OpenMode::Read).unwrap();

        let nb_sample = s.get_info().channels as i64 * s.get_info().frames;
        let mut samples = vec![0i16; nb_sample as usize];
        s.read_i16(samples.as_mut_slice(), nb_sample as i64);

        // If we get a stereo file, convert it to mono
        if s.get_info().channels == 2 {
            samples = convert_to_mono(samples);
        }

        (samples, s.get_info().samplerate)
    }
}
