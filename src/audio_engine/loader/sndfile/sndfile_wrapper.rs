// The MIT License (MIT)
//
// Copyright (c) 2013 Jeremy Letang (letang.jeremy@gmail.com)
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
// FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
// COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
// IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

/*!
* Libsndfile is a library designed to allow the reading and writing of many
* different sampled sound file formats (such as MS Windows WAV and
* the Apple/SGI AIFF format) through one standard library interface.
*
* During read and write operations, formats are seamlessly converted between the
* format the application program has requested or supplied and the file's data
* format. The application programmer can remain blissfully unaware of issues
* such as file endian-ness and data format
*/

#![allow(dead_code)]

use std::ffi::{CStr, CString};
use std::ops::BitOr;
use std::ptr;

use sndfile_sys as ffi;

/// The SndInfo structure is for passing data between the calling
/// function and the library when opening a file for reading or writing.
#[repr(C)]
#[derive(Clone)]
pub struct SndInfo {
    pub frames: i64,
    pub samplerate: i32,
    pub channels: i32,
    pub format: i32,
    pub sections: i32,
    pub seekable: i32,
}

/// Modes availables for the open function.
///
/// * Read - Read only mode
/// * Write - Write only mode
/// * ReadWrite - Read and Write mode
#[derive(Clone)]
pub enum OpenMode {
    Read = ffi::SFM_READ as isize,
    Write = ffi::SFM_WRITE as isize,
    ReadWrite = ffi::SFM_RDWR as isize,
}

/// Type of strings available for method get_string()
#[derive(Clone)]
pub enum StringSoundType {
    Title = ffi::SF_STR_TITLE as isize,
    Copyright = ffi::SF_STR_COPYRIGHT as isize,
    Software = ffi::SF_STR_SOFTWARE as isize,
    Artist = ffi::SF_STR_ARTIST as isize,
    Comment = ffi::SF_STR_COMMENT as isize,
    Date = ffi::SF_STR_DATE as isize,
    Album = ffi::SF_STR_ALBUM as isize,
    License = ffi::SF_STR_LICENSE as isize,
    TrackNumber = ffi::SF_STR_TRACKNUMBER as isize,
    Genre = ffi::SF_STR_GENRE as isize,
}

/// Types of error who can be return by API functions
#[repr(C)]
#[derive(Clone)]
pub enum Error {
    NoError = ffi::SF_ERR_NO_ERROR as isize,
    UnrecognizedFormat = ffi::SF_ERR_UNRECOGNISED_FORMAT as isize,
    SystemError = ffi::SF_ERR_SYSTEM as isize,
    MalformedFile = ffi::SF_ERR_MALFORMED_FILE as isize,
    UnsupportedEncoding = ffi::SF_ERR_UNSUPPORTED_ENCODING as isize,
    UnknownError = -1,
}

impl Error {
    pub fn from_i32(code: i32) -> Error {
        match code {
            ffi::SF_ERR_NO_ERROR => Error::NoError,
            ffi::SF_ERR_UNRECOGNISED_FORMAT => Error::UnrecognizedFormat,
            ffi::SF_ERR_SYSTEM => Error::SystemError,
            ffi::SF_ERR_MALFORMED_FILE => Error::MalformedFile,
            ffi::SF_ERR_UNSUPPORTED_ENCODING => Error::UnsupportedEncoding,
            _ => Error::UnknownError,
        }
    }
}

/// Enum to set the offset with method seek
///
/// * SeekSet - The offset is set to the start of the audio data plus offset (multichannel) frames.
/// * SeekCur - The offset is set to its current location plus offset (multichannel) frames.
/// * SeekEnd - The offset is set to the end of the data plus offset (multichannel) frames.
#[derive(Clone)]
pub enum SeekMode {
    SeekSet = ffi::SF_SEEK_SET as isize,
    SeekCur = ffi::SF_SEEK_CUR as isize,
    SeekEnd = ffi::SF_SEEK_END as isize,
}

/// Enum who contains the list of the supported audio format
///
/// * FormatWav - Microsoft WAV format (little endian)
/// * FormatAiff - Apple/SGI AIFF format (big endian)
/// * FormatAu - Sun/NeXT AU format (big endian)
/// * FormatRaw - RAW PCM data
/// * FormatPaf - Ensoniq PARIS file format
/// * FormatSvx - Amiga IFF / SVX8 / SV16 format
/// * FormatNist - Sphere NIST format
/// * FormatVoc - VOC files
/// * FormatIrcam - Berkeley/IRCAM/CARL
/// * FormatW64 - Sonic Foundry's 64 bit RIFF/WAV
/// * FormatMat4 - Matlab (tm) V4.2 / GNU Octave 2.0
/// * FormatMat5 - Matlab (tm) V5.0 / GNU Octave 2.1
/// * FormatPvf - Portable Voice Format
/// * FormatXi - Fasttracker 2 Extended Instrument
/// * FormatHtk - HMM Tool Kit format
/// * FormatSds - Midi Sample Dump Standard
/// * FormatAvr - Audio Visual Research
/// * FormatWavex - MS WAVE with WAVEFORMATEX
/// * FormatSd2 - Sound Designer 2
/// * FormatFlac - FLAC lossless file format
/// * FormatCaf - Core Audio File format
/// * FormatWve - Psion WVE format
/// * FormatOgg - Xiph OGG container
/// * FormatMpc2k - Akai MPC 2000 sampler
/// * FormatRf64 - RF64 WAV file
/// * FormatPcmS8 - Signed 8 bit data
/// * FormatPcm16 - Signed 16 bit data
/// * FormatPcm24 - Signed 24 bit data
/// * FormatPcm32 - Signed 32 bit data
/// * FormatPcmU8 - Unsigned 8 bit data (WAV and RAW only)
/// * FormatFloat - 32 bit float data
/// * FormatDouble - 64 bit float data
/// * FormatUlaw - U-Law encoded
/// * FormatAlaw - A-Law encoded
/// * FormatImaAdpcm - IMA ADPCM
/// * FormatApcm - Microsoft ADPCM
/// * FormatGsm610 - GSM 6.10 encoding
/// * FormatVoxAdpcm - Oki Dialogic ADPCM encoding
/// * FormatG72132 - 32kbs G721 ADPCM encoding
/// * FormatG72324 - 24kbs G723 ADPCM encoding
/// * FormatG72340 - 40kbs G723 ADPCM encoding
/// * FormatDww12 - 12 bit Delta Width Variable Word encoding
/// * FormatDww16 - 16 bit Delta Width Variable Word encoding
/// * FormatDww24 - 24 bit Delta Width Variable Word encoding
/// * FormatDwwN - N bit Delta Width Variable Word encoding
/// * FormatDpcm8 - 8 bit differential PCM (XI only)
/// * FormatDpcm16 - 16 bit differential PCM (XI only)
/// * FormatVorbis - Xiph Vorbis encoding
/// * EndianFile - Default file endian-ness
/// * EndianLittle - Force little endian-ness
/// * EndianBig - Force big endian-ness
/// * EndianCpu - Force CPU endian-ness
#[repr(C)]
#[derive(Clone, PartialOrd, PartialEq, Copy)]
pub enum FormatType {
    FormatWav = ffi::SF_FORMAT_WAV as isize,
    FormatAiff = ffi::SF_FORMAT_AIFF as isize,
    FormatAu = ffi::SF_FORMAT_AU as isize,
    FormatRaw = ffi::SF_FORMAT_RAW as isize,
    FormatPaf = ffi::SF_FORMAT_PAF as isize,
    FormatSvx = ffi::SF_FORMAT_SVX as isize,
    FormatNist = ffi::SF_FORMAT_NIST as isize,
    FormatVoc = ffi::SF_FORMAT_VOC as isize,
    FormatIrcam = ffi::SF_FORMAT_IRCAM as isize,
    FormatW64 = ffi::SF_FORMAT_W64 as isize,
    FormatMat4 = ffi::SF_FORMAT_MAT4 as isize,
    FormatMat5 = ffi::SF_FORMAT_MAT5 as isize,
    FormatPvf = ffi::SF_FORMAT_PVF as isize,
    FormatXi = ffi::SF_FORMAT_XI as isize,
    FormatHtk = ffi::SF_FORMAT_HTK as isize,
    FormatSds = ffi::SF_FORMAT_SDS as isize,
    FormatAvr = ffi::SF_FORMAT_AVR as isize,
    FormatWavex = ffi::SF_FORMAT_WAVEX as isize,
    FormatSd2 = ffi::SF_FORMAT_SD2 as isize,
    FormatFlac = ffi::SF_FORMAT_FLAC as isize,
    FormatCaf = ffi::SF_FORMAT_CAF as isize,
    FormatWve = ffi::SF_FORMAT_WVE as isize,
    FormatOgg = ffi::SF_FORMAT_OGG as isize,
    FormatMpc2k = ffi::SF_FORMAT_MPC2K as isize,
    FormatRf64 = ffi::SF_FORMAT_RF64 as isize,
    FormatPcmS8 = ffi::SF_FORMAT_PCM_S8 as isize,
    FormatPcm16 = ffi::SF_FORMAT_PCM_16 as isize,
    FormatPcm24 = ffi::SF_FORMAT_PCM_24 as isize,
    FormatPcm32 = ffi::SF_FORMAT_PCM_32 as isize,
    FormatPcmU8 = ffi::SF_FORMAT_PCM_U8 as isize,
    FormatFloat = ffi::SF_FORMAT_FLOAT as isize,
    FormatDouble = ffi::SF_FORMAT_DOUBLE as isize,
    FormatUlaw = ffi::SF_FORMAT_ULAW as isize,
    FormatAlaw = ffi::SF_FORMAT_ALAW as isize,
    FormatImaAdpcm = ffi::SF_FORMAT_IMA_ADPCM as isize,
    FormatApcm = ffi::SF_FORMAT_MS_ADPCM as isize,
    FormatGsm610 = ffi::SF_FORMAT_GSM610 as isize,
    FormatVoxAdpcm = ffi::SF_FORMAT_VOX_ADPCM as isize,
    FormatG72132 = ffi::SF_FORMAT_G721_32 as isize,
    FormatG72324 = ffi::SF_FORMAT_G723_24 as isize,
    FormatG72340 = ffi::SF_FORMAT_G723_40 as isize,
    FormatDww12 = ffi::SF_FORMAT_DWVW_12 as isize,
    FormatDww16 = ffi::SF_FORMAT_DWVW_16 as isize,
    FormatDww24 = ffi::SF_FORMAT_DWVW_24 as isize,
    FormatDwwN = ffi::SF_FORMAT_DWVW_N as isize,
    FormatDpcm8 = ffi::SF_FORMAT_DPCM_8 as isize,
    FormatDpcm16 = ffi::SF_FORMAT_DPCM_16 as isize,
    FormatVorbis = ffi::SF_FORMAT_VORBIS as isize,
    EndianFile = ffi::SF_ENDIAN_FILE as isize,
    EndianLittle = ffi::SF_ENDIAN_LITTLE as isize,
    EndianBig = ffi::SF_ENDIAN_BIG as isize,
    EndianCpu = ffi::SF_ENDIAN_CPU as isize,
    FormatSubMask = ffi::SF_FORMAT_SUBMASK as isize,
    FormatTypeMask = ffi::SF_FORMAT_TYPEMASK as isize,
}

impl BitOr for FormatType {
    type Output = isize;

    fn bitor(self, _rhs: FormatType) -> isize {
        (self as isize) | (_rhs as isize)
    }
}

/// SndFile object, used to load/store sound from a file path or an fd.
pub struct SndFile {
    handle: *mut ffi::SNDFILE,
    info: Box<ffi::SF_INFO>,
}

impl Clone for SndFile {
    fn clone(&self) -> SndFile {
        SndFile {
            handle: self.handle,
            info: self.info.clone(),
        }
    }
}

impl SndFile {
    /**
     * Construct SndFile object with the path to the music and a mode to open it.
     *
     * # Arguments
     * * path - The path to load the music
     * * mode - The mode to open the music
     *
     * Return Ok() containing the SndFile on success, a string representation of
     * the error otherwise.
     */
    pub fn new(path: &str, mode: OpenMode) -> Result<SndFile, String> {
        let mut info = Box::new(ffi::SF_INFO {
            frames: 0,
            samplerate: 0,
            channels: 0,
            format: 0,
            sections: 0,
            seekable: 0,
        });

        let path_c = CString::new(path).unwrap();
        let tmp_sndfile = unsafe { ffi::sf_open(path_c.into_raw(), mode as i32, &mut *info) };
        if tmp_sndfile.is_null() {
            Err(unsafe {
                CStr::from_ptr(ffi::sf_strerror(ptr::null_mut()))
                    .to_str()
                    .unwrap()
                    .to_owned()
            })
        } else {
            Ok(SndFile {
                handle: tmp_sndfile,
                info: info,
            })
        }
    }

    /**
     * Construct SndFile object with the path to the music and a mode to open it.
     *
     * # Arguments
     * * path - The path to load the music
     * * mode - The mode to open the music
     * * info - The SndInfo to pass to the file
     *
     * Return Ok() containing the SndFile on success, a string representation of
     * the error otherwise.
     */
    pub fn new_with_info(
        path: &str,
        mode: OpenMode,
        mut info: Box<ffi::SF_INFO>,
    ) -> Result<SndFile, String> {
        let path_c = CString::new(path).unwrap();
        let tmp_sndfile = unsafe { ffi::sf_open(path_c.into_raw(), mode as i32, &mut *info) };
        if tmp_sndfile.is_null() {
            Err(unsafe {
                CStr::from_ptr(ffi::sf_strerror(ptr::null_mut()))
                    .to_str()
                    .unwrap()
                    .to_owned()
            })
        } else {
            Ok(SndFile {
                handle: tmp_sndfile,
                info: info,
            })
        }
    }

    /**
     * Construct SndFile object with the fd of the file containing the music
     * and a mode to open it.
     *
     * # Arguments
     * * fd - The fd to load the music
     * * mode - The mode to open the music
     * * close_desc - Should SndFile close the fd at exit?
     *
     * Return Ok() containing the SndFile on success, a string representation
     * of the error otherwise.
     */
    pub fn new_with_fd(fd: i32, mode: OpenMode, close_desc: bool) -> Result<SndFile, String> {
        let mut info = Box::new(ffi::SF_INFO {
            frames: 0,
            samplerate: 0,
            channels: 0,
            format: 0,
            sections: 0,
            seekable: 0,
        });
        let tmp_sndfile = if close_desc {
            unsafe { ffi::sf_open_fd(fd, mode as i32, &mut *info, ffi::SF_TRUE) }
        } else {
            unsafe { ffi::sf_open_fd(fd, mode as i32, &mut *info, ffi::SF_FALSE) }
        };
        if tmp_sndfile.is_null() {
            Err(unsafe {
                CStr::from_ptr(ffi::sf_strerror(ptr::null_mut()))
                    .to_str()
                    .unwrap()
                    .to_owned()
            })
        } else {
            Ok(SndFile {
                handle: tmp_sndfile,
                info: info,
            })
        }
    }

    /// Return the SndInfo struct of the current music.
    pub fn get_info(&self) -> ffi::SF_INFO {
        *self.info.clone()
    }

    /**
     * Retrieve a tag contained by the music.
     *
     * # Argument
     * * string_type - The type of the tag to retrieve
     *
     * Return Some(String) if the tag is found, None otherwise.
     */
    pub fn get_string(&self, string_type: StringSoundType) -> Option<String> {
        let c_string = unsafe { ffi::sf_get_string(self.handle, string_type as i32) };
        if c_string.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(c_string).to_str().unwrap().to_owned() })
        }
    }

    /**
     * Set a tag on the music file.
     *
     * # Arguments
     * * string_type - The type of the tag to set
     * * string - The string to set.
     *
     * Return NoError on success, an other error code otherwise
     */
    pub fn set_string(&mut self, string_type: StringSoundType, string: String) -> Error {
        let string_c = CString::new(string).unwrap();
        Error::from_i32(unsafe {
            ffi::sf_set_string(self.handle, string_type as i32, string_c.into_raw())
        })
    }

    /**
     * Check if the format of the SndInfo struct is valid.
     *
     * # Argument
     * * info - The SndInfo struct to test
     *
     * Return true if the struct is valid, false otherwise.
     */
    pub fn check_format(info: &mut ffi::SF_INFO) -> bool {
        match unsafe { ffi::sf_format_check(info) } {
            ffi::SF_TRUE => true,
            ffi::SF_FALSE => false,
            _ => unreachable!(),
        }
    }

    /**
     * Close the SndFile object.
     *
     * This function must be called before the exist of the program to destroy
     * all the resources.
     *
     * Return NoError if destruction success, an other error code otherwise.
     */
    pub fn close(&self) -> Error {
        Error::from_i32(unsafe { ffi::sf_close(self.handle) })
    }

    /**
     * If the file is opened Write or ReadWrite, call the operating system's
     * function to force the writing of all file cache buffers to disk.
     * If the file is opened Read no action is taken.
     */
    pub fn write_sync(&mut self) -> () {
        unsafe { ffi::sf_write_sync(self.handle) }
    }

    pub fn seek(&mut self, frames: i64, whence: SeekMode) -> i64 {
        unsafe { ffi::sf_seek(self.handle, frames, whence as i32) }
    }

    /**
     * Read items of type i16
     *
     * # Arguments
     * * array - The array to fill with the items.
     * * items - The max capacity of the array.
     *
     * Return the count of items.
     */
    pub fn read_i16<'r>(&'r mut self, array: &'r mut [i16], items: i64) -> i64 {
        unsafe { ffi::sf_read_short(self.handle, array.as_mut_ptr(), items) }
    }

    /**
     * Read items of type i32
     *
     * # Arguments
     * * array - The array to fill with the items.
     * * items - The max capacity of the array.
     *
     * Return the count of items.
     */
    pub fn read_i32<'r>(&'r mut self, array: &'r mut [i32], items: i64) -> i64 {
        unsafe { ffi::sf_read_int(self.handle, array.as_mut_ptr(), items) }
    }

    /**
     * Read items of type f32
     *
     * # Arguments
     * * array - The array to fill with the items.
     * * items - The max capacity of the array.
     *
     * Return the count of items.
     */
    pub fn read_f32<'r>(&'r mut self, array: &'r mut [f32], items: i64) -> i64 {
        unsafe { ffi::sf_read_float(self.handle, array.as_mut_ptr(), items) }
    }

    /**
     * Read items of type f64
     *
     * # Arguments
     * * array - The array to fill with the items.
     * * items - The max capacity of the array.
     *
     * Return the count of items.
     */
    pub fn read_f64<'r>(&'r mut self, array: &'r mut [f64], items: i64) -> i64 {
        unsafe { ffi::sf_read_double(self.handle, array.as_mut_ptr(), items) }
    }

    /**
     * Read frames of type i16
     *
     * # Arguments
     * * array - The array to fill with the frames.
     * * items - The max capacity of the array.
     *
     * Return the count of frames.
     */
    pub fn readf_i16<'r>(&'r mut self, array: &'r mut [i16], frames: i64) -> i64 {
        unsafe { ffi::sf_readf_short(self.handle, array.as_mut_ptr(), frames) }
    }

    /**
     * Read frames of type i32
     *
     * # Arguments
     * * array - The array to fill with the frames.
     * * items - The max capacity of the array.
     *
     * Return the count of frames.
     */
    pub fn readf_i32<'r>(&'r mut self, array: &'r mut [i32], frames: i64) -> i64 {
        unsafe { ffi::sf_readf_int(self.handle, array.as_mut_ptr(), frames) }
    }

    /**
     * Read frames of type f32
     *
     * # Arguments
     * * array - The array to fill with the frames.
     * * items - The max capacity of the array.
     *
     * Return the count of frames.
     */
    pub fn readf_f32<'r>(&'r mut self, array: &'r mut [f32], frames: i64) -> i64 {
        unsafe { ffi::sf_readf_float(self.handle, array.as_mut_ptr(), frames) }
    }

    /**
     * Read frames of type f64
     *
     * # Arguments
     * * array - The array to fill with the frames.
     * * items - The max capacity of the array.
     *
     * Return the count of frames.
     */
    pub fn readf_f64<'r>(&'r mut self, array: &'r mut [f64], frames: i64) -> i64 {
        unsafe { ffi::sf_readf_double(self.handle, array.as_mut_ptr(), frames) }
    }

    /**
     * Write items of type i16
     *
     * # Arguments
     * * array - The array of items to write.
     * * items - The number of items to write.
     *
     * Return the count of wrote items.
     */
    pub fn write_i16<'r>(&'r mut self, array: &'r mut [i16], items: i64) -> i64 {
        unsafe { ffi::sf_write_short(self.handle, array.as_mut_ptr(), items) }
    }

    /**
     * Write items of type i32
     *
     * # Arguments
     * * array - The array of items to write.
     * * items - The number of items to write.
     *
     * Return the count of wrote items.
     */
    pub fn write_i32<'r>(&'r mut self, array: &'r mut [i32], items: i64) -> i64 {
        unsafe { ffi::sf_write_int(self.handle, array.as_mut_ptr(), items) }
    }

    /**
     * Write items of type f32
     *
     * # Arguments
     * * array - The array of items to write.
     * * items - The number of items to write.
     *
     * Return the count of wrote items.
     */
    pub fn write_f32<'r>(&'r mut self, array: &'r mut [f32], items: i64) -> i64 {
        unsafe { ffi::sf_write_float(self.handle, array.as_mut_ptr(), items) }
    }

    /**
     * Write items of type f64
     *
     * # Arguments
     * * array - The array of items to write.
     * * items - The number of items to write.
     *
     * Return the count of wrote items.
     */
    pub fn write_f64<'r>(&'r mut self, array: &'r mut [f64], items: i64) -> i64 {
        unsafe { ffi::sf_write_double(self.handle, array.as_mut_ptr(), items) }
    }

    /**
     * Write frames of type i16
     *
     * # Arguments
     * * array - The array of frames to write.
     * * items - The number of frames to write.
     *
     * Return the count of wrote frames.
     */
    pub fn writef_i16<'r>(&'r mut self, array: &'r mut [i16], frames: i64) -> i64 {
        unsafe { ffi::sf_writef_short(self.handle, array.as_mut_ptr(), frames) }
    }

    /**
     * Write frames of type i32
     *
     * # Arguments
     * * array - The array of frames to write.
     * * items - The number of frames to write.
     *
     * Return the count of wrote frames.
     */
    pub fn writef_i32<'r>(&'r mut self, array: &'r mut [i32], frames: i64) -> i64 {
        unsafe { ffi::sf_writef_int(self.handle, array.as_mut_ptr(), frames) }
    }

    /**
     * Write frames of type f32
     *
     * # Arguments
     * * array - The array of frames to write.
     * * items - The number of frames to write.
     *
     * Return the count of wrote frames.
     */
    pub fn writef_f32<'r>(&'r mut self, array: &'r mut [f32], frames: i64) -> i64 {
        unsafe { ffi::sf_writef_float(self.handle, array.as_mut_ptr(), frames) }
    }

    /**
     * Write frames of type f64
     *
     * # Arguments
     * * array - The array of frames to write.
     * * items - The number of frames to write.
     *
     * Return the count of wrote frames.
     */
    pub fn writef_f64<'r>(&'r mut self, array: &'r mut [f64], frames: i64) -> i64 {
        unsafe { ffi::sf_writef_double(self.handle, array.as_mut_ptr(), frames) }
    }

    /**
     * Get the last error
     *
     * Return the last error as a variant of the enum Error.
     */
    pub fn error(&self) -> Error {
        Error::from_i32(unsafe { ffi::sf_error(self.handle) })
    }

    /**
     * Get the last error as a string
     *
     * Return an owned str containing the last error.
     */
    pub fn string_error(&self) -> String {
        unsafe {
            CStr::from_ptr(ffi::sf_strerror(self.handle))
                .to_str()
                .unwrap()
                .to_owned()
        }
    }

    /**
     * Get an error as a string from a variant of enum Error
     *
     * Return an owned str containing the error.
     */
    pub fn error_number(error_num: Error) -> String {
        unsafe {
            CStr::from_ptr(ffi::sf_error_number(error_num as i32))
                .to_str()
                .unwrap()
                .to_owned()
        }
    }
}
