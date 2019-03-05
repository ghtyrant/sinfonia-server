#![warn(unused_extern_crates)]

extern crate alto;
extern crate env_logger;
extern crate futures;
extern crate gotham;
extern crate gotham_serde_json_body_parser;
extern crate hyper;
extern crate minimp3;
extern crate rand;
extern crate serde;
extern crate sndfile_sys;
extern crate unicase;

extern crate structopt;
#[macro_use]
extern crate log;
extern crate failure;
#[macro_use]
extern crate serde_derive;

#[macro_use]
mod utils;
#[macro_use]
mod audio_engine;
mod api;
mod authorization;
mod error;
mod theme;

use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::thread;

use structopt::StructOpt;

use api::start_web_service;
use audio_engine::backends::alto::OpenALBackend;
use audio_engine::engine::start_audio_controller;
use audio_engine::messages::command;

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

    #[structopt(
        short = "s",
        long = "sound-library",
        default_value = "/home/fabian/tmp/sound/",
        parse(from_os_str)
    )]
    sound_library: PathBuf,
}

fn main() {
    std::env::set_var("RUST_LOG", "sinfonia_server=debug,alto=debug");
    std::env::set_var("RUST_BACKTRACE", "full");

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
    let (response_sender, response_receiver) = channel();

    let handle = thread::spawn(|| {
        start_audio_controller::<OpenALBackend>(receiver, response_sender, library_path)
    });
    let main_sender = sender.clone();

    // This does not return until done
    start_web_service(opt.host, opt.threads, &sender, response_receiver, opt.token);

    // Tell AudioController to shut down
    main_sender
        .send(build_command!(Quit))
        .expect("Failed to send AudioControllerMessage::Quit to AudioController!");

    // Wait until AudioController shuts down
    handle
        .join()
        .expect("Waiting for the AudioController to finish has failed!");
}
