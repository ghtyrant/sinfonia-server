use sndfile_sys;

use std::ffi::{CStr, CString};
use std::path::PathBuf;
use std::ptr;

use audio_engine::loader::base::AudioFileLoader;
use audio_engine::loader::error::AudioFileLoaderError;
use utils::convert_to_mono;

pub struct SndFileLoader;

#[link(name = "libsndfile-1")]
extern "C" {}

impl AudioFileLoader for SndFileLoader {
    fn load(&mut self, path: &PathBuf) -> Result<(Vec<i16>, i32), AudioFileLoaderError> {
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
            return Err(AudioFileLoaderError::FileLoadError(
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
