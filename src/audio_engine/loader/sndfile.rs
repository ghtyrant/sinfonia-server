use sndfile_sys;

use itertools::Itertools;
use num::{Integer, NumCast, PrimInt};
use std::ffi::{CStr, CString};
use std::path::PathBuf;
use std::ptr;

use audio_engine::loader::base::AudioFileLoader;
use error::SinfoniaGenericError;

pub struct SndFileLoader;

#[link(name = "libsndfile-1")]
extern "C" {}

fn convert_to_mono(samples: Vec<i16>) -> Vec<i16> {
    samples
        .into_iter()
        .chunks(2)
        .into_iter()
        // We use fold() instead of sum() here so we are able to upcast to i32
        // before adding and thus avoid overflow errors
        .map::<i16, _>(|a| (a.fold::<i32, _>(0i32, |acc, x| acc + x as i32) / 2) as i16)
        .collect()
}

impl AudioFileLoader for SndFileLoader {
    fn load(&mut self, path: &PathBuf) -> Result<(Vec<i16>, i32), SinfoniaGenericError> {
        let mut info = Box::new(sndfile_sys::SF_INFO {
            frames: 0,
            samplerate: 0,
            channels: 0,
            format: 0,
            sections: 0,
            seekable: 0,
        });

        let path_c = CString::new(path.to_str().unwrap()).unwrap();
        let tmp_sndfile =
            unsafe { sndfile_sys::sf_open(path_c.into_raw(), sndfile_sys::SFM_READ, &mut *info) };

        if tmp_sndfile.is_null() {
            return Err(SinfoniaGenericError::FileLoadError(
                path.to_string_lossy().into_owned(),
                unsafe {
                    CStr::from_ptr(sndfile_sys::sf_strerror(ptr::null_mut()))
                        .to_str()
                        .unwrap()
                        .to_owned()
                },
            ));
        }

        let len = info.channels as i64 * info.frames;
        let mut samples = vec![0i16; len as usize];
        unsafe {
            sndfile_sys::sf_read_short(
                tmp_sndfile,
                samples.as_mut_slice().as_mut_ptr(),
                len as i64,
            );
        }

        // If we get a stereo file, convert it to mono
        if info.channels == 2 {
            samples = convert_to_mono(samples);
        }

        Ok((samples, info.samplerate))
    }
}
