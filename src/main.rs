extern crate env_logger;
extern crate failure;
extern crate futures;
extern crate gotham;
extern crate gotham_serde_json_body_parser;
extern crate hyper;
extern crate rand;
extern crate rfmod;
extern crate serde;
extern crate serde_json;
extern crate unicase;
extern crate alto;
extern crate sndfile_sys;
extern crate num;
extern crate itertools;

extern crate structopt;
#[macro_use]
extern crate log;
#[macro_use]
extern crate quick_error;
#[macro_use]
extern crate serde_derive;

#[macro_use]
mod utils;
mod api;
mod audio_engine;
mod authorization;
mod error;
mod sound_funcs;
mod theme;

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::thread;

use structopt::StructOpt;

use api::start_web_service;
use audio_engine::engine::start_audio_controller;
use audio_engine::messages::command;
use audio_engine::backends::alto::OpenALAudioBackend;

/// A basic example
#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    #[structopt(short = "h", long = "host", default_value = "127.0.0.1:9090")]
    host: String,

    #[structopt(
        short = "a",
        long = "access-token",
        default_value = "totallynotsecure"
    )]
    token: String,

    #[structopt(short = "t", long = "threads", default_value = "2")]
    threads: usize,

    #[structopt(
        short = "s",
        long = "sound-library",
        default_value = "/home/fabian/tmp/sound/",
        parse(from_os_str)
    )]
    sound_library: PathBuf,
}

fn main() {
    std::env::set_var("RUST_LOG", "sinfonia_server=debug");
    std::env::set_var("RUST_BACKTRACE", "1");

    let opt = Opt::from_args();

    env_logger::init();
    info!("Starting up!");

    // Start server
    info!(
        "Starting server on {}, threads: {}, access token: '{}', sound library: '{}'",
        opt.host,
        opt.threads,
        opt.token,
        opt.sound_library.to_string_lossy()
    );

    let library_path = opt.sound_library.clone();

    // Set up channel for REST->AudioController communication
    let (sender, receiver) = channel();

    let handle = thread::spawn(|| start_audio_controller::<OpenALAudioBackend>(receiver, library_path));
    let main_sender = sender.clone();

    // This does not return until done
    start_web_service(opt.host, opt.threads, &sender, opt.token);

    // Tell AudioController to shut down
    let cmd: command::Command = command::Quit::init()(sender);
    main_sender
        .send(cmd)
        .expect("Failed to send AudioControllerMessage::Quit to AudioController!");
    
    // Wait until AudioController shuts down
    handle
        .join()
        .expect("Waiting for the AudioController to finish has failed!");
}
