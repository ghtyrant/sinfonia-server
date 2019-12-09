#![warn(unused_extern_crates)]

#[macro_use]
extern crate rusqlite;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

#[macro_use]
mod utils;
#[macro_use]
mod audio_engine;
mod api;
mod authorization;
mod error;
mod samplesdb;
mod theme;

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use structopt::StructOpt;

use api::start_web_service;
use audio_engine::backends::alto::OpenALBackend;
use audio_engine::engine::start_audio_controller;
use audio_engine::messages::{Command, Response};
use samplesdb::{SamplesDB, SamplesDBError};

/// A basic example
#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    #[structopt(short = "h", long = "host", default_value = "127.0.0.1")]
    host: String,

    #[structopt(short = "p", long = "port", default_value = "9090")]
    port: u32,

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

pub type ChannelSender = Sender<Command>;
pub type ResponseReceiver = Receiver<Response>;

#[actix_rt::main]
async fn main() -> Result<(), SamplesDBError> {
    std::env::set_var(
        "RUST_LOG",
        "sinfonia_server=debug,alto=debug,actix_web=debug",
    );
    std::env::set_var("RUST_BACKTRACE", "full");

    let opt = Opt::from_args();

    env_logger::init();
    info!("Starting up!");

    // Start server
    info!(
        "Starting server on {}:{}, threads: {}, access token: '{}', sound library: '{}'",
        opt.host,
        opt.port,
        opt.threads,
        opt.token,
        opt.sound_library.to_string_lossy()
    );

    let library_path = opt.sound_library.clone();

    // Set up channel for REST->AudioController communication
    let (sender, receiver) = channel();
    let (response_sender, response_receiver) = channel();

    let samplesdb = SamplesDB::open(Path::new("samples.db"), &library_path)?;
    let handle = thread::spawn(|| {
        start_audio_controller::<OpenALBackend>(receiver, response_sender, samplesdb)
    });
    let main_sender = sender.clone();

    match start_web_service(
        opt.host,
        opt.port,
        main_sender.clone(),
        response_receiver,
        opt.token,
    )
    .await
    {
        Ok(_) => {}
        Err(e) => panic!("Failed to bind webserver: {}!", e),
    }

    // Tell AudioController to shut down:
    main_sender
        .send(Command::Quit)
        .expect("Failed to send AudioControllerMessage::Quit to AudioController!");

    // Wait until AudioController shuts down
    handle
        .join()
        .expect("Waiting for the AudioController to finish has failed!");

    Ok(())
}
