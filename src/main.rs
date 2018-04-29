extern crate env_logger;
extern crate futures;
extern crate gotham;
extern crate gotham_serde_json_body_parser;
extern crate hyper;
extern crate rand;
extern crate rfmod;
extern crate serde;
extern crate serde_json;
extern crate unicase;
extern crate failure;

#[macro_use]
extern crate structopt;
#[macro_use]
extern crate log;
#[macro_use]
extern crate quick_error;
#[macro_use]
extern crate serde_derive;

#[macro_use]
mod utils;
mod authorization;
mod api;
mod theme;
mod error;
mod audio;
mod sound_funcs;

use std::sync::mpsc::channel;
use std::thread;
use std::path::PathBuf;

use structopt::StructOpt;

use audio::{start_audio_controller, AudioControllerMessage};
use api::start_web_service;

/// A basic example
#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    #[structopt(short = "h", long = "host", default_value = "127.0.0.1:9090")]
    host: String,

    #[structopt(short = "a", long = "access-token", default_value = "totallynotsecure")]
    token: String,

    #[structopt(short = "t", long = "threads", default_value = "2")]
    threads: usize,

    #[structopt(short = "s", long = "sound-library", default_value = ".", parse(from_os_str))]
    sound_library: PathBuf,
}

fn main() {
    std::env::set_var("RUST_LOG", "sinfonia_server=debug");
    std::env::set_var("RUST_BACKTRACE", "1");

    let opt = Opt::from_args();

    env_logger::init();
    info!("Starting up!");

    // Set up channel for REST->AudioController communication
    let (sender, receiver) = channel();

    // Start server
    info!(
        "Starting server on {}, threads: {}, access token: '{}', sound library: '{}'",
        opt.host, opt.threads, opt.token, opt.sound_library.to_string_lossy()
    );

    let library_path = opt.sound_library.clone();
    let handle = thread::spawn(|| start_audio_controller(receiver, library_path));
    let main_sender = sender.clone();

    start_web_service(opt.host, opt.threads, &sender, opt.token);
    main_sender.send(AudioControllerMessage::Quit {}).expect("Failed to send AudioControllerMessage::Quit to AudioController!");
    handle.join().expect("Waiting for the AudioController to finish has failed!");
}
