use std::io;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{get, post, web, App, HttpResponse, HttpServer};

use crate::audio_engine::messages::{Command, Response};
use crate::authorization::TokenAuthorization;
use crate::theme::Theme;

pub type ChannelSender = Sender<Command>;
pub type ResponseReceiver = Receiver<Response>;

pub mod api_response {
    use std::collections::HashMap;

    #[derive(Serialize)]
    pub struct Error {
        pub message: String,
    }

    #[derive(Serialize)]
    pub struct Status {
        pub playing: bool,
        pub theme_loaded: bool,
        pub theme: Option<String>,
        pub sounds_playing: Vec<String>,
        pub sounds_playing_next: HashMap<String, u64>,
        pub previewing: Vec<String>,
    }

    #[derive(Serialize)]
    pub struct SoundLibrary {
        pub samples: Vec<String>,
    }
}

struct APIData {
    sender: ChannelSender,
    receiver: ResponseReceiver,
}

impl APIData {
    fn new(sender: ChannelSender, receiver: ResponseReceiver) -> Self {
        Self { sender, receiver }
    }
}

type APIDataType = web::Data<Arc<Mutex<APIData>>>;

macro_rules! send_message {
    ($sender: expr, $receiver: expr, $response: path, $message: expr) => {{
        $sender
            .send($message)
            .expect("Failed to communicate with audio engine!");

        match $receiver.recv() {
            Ok(r) => match r {
                Response::Error { message } => Err(message),
                $response { .. } => Ok(r),
                _ => panic!("Internal Error!"),
            },
            Err(_) => panic!("Internal Error!"),
        }
    }};

    ($sender: expr, $receiver: expr, $message: expr) => {{
        send_message!($sender, $receiver, Response::Success, $message)
    }};
}

#[post("/pause")]
async fn pause(state: APIDataType) -> HttpResponse {
    let api_data = state.lock().unwrap();

    match send_message!(api_data.sender, api_data.receiver, Command::Pause) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(message) => HttpResponse::BadRequest().json(api_response::Error { message }),
    }
}

#[post("/play")]
async fn play(state: APIDataType) -> HttpResponse {
    let api_data = state.lock().unwrap();

    match send_message!(api_data.sender, api_data.receiver, Command::Play) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(message) => HttpResponse::BadRequest().json(api_response::Error { message }),
    }
}

#[derive(Deserialize)]
struct PreviewSound {
    name: String,
}

#[post("/preview")]
async fn preview(state: APIDataType, payload: web::Json<PreviewSound>) -> HttpResponse {
    let api_data = state.lock().unwrap();
    match send_message!(
        api_data.sender,
        api_data.receiver,
        Command::PreviewSound {
            sound: payload.name.clone()
        }
    ) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(message) => HttpResponse::BadRequest().json(api_response::Error { message }),
    }
}

#[post("/theme")]
async fn theme(state: APIDataType, payload: web::Json<Theme>) -> HttpResponse {
    let api_data = state.lock().unwrap();
    match send_message!(
        api_data.sender,
        api_data.receiver,
        Command::LoadTheme {
            theme: payload.into_inner()
        }
    ) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(message) => HttpResponse::BadRequest().json(api_response::Error { message }),
    }
}

#[derive(Deserialize)]
struct Trigger {
    name: String,
}

#[post("/trigger")]
async fn trigger(state: APIDataType, payload: web::Json<Trigger>) -> HttpResponse {
    let api_data = state.lock().unwrap();
    match send_message!(
        api_data.sender,
        api_data.receiver,
        Command::Trigger {
            sound: payload.name.clone()
        }
    ) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(message) => HttpResponse::BadRequest().json(api_response::Error { message }),
    }
}

#[get("/status")]
async fn status(state: APIDataType) -> HttpResponse {
    let api_data = state.lock().unwrap();

    match send_message!(
        api_data.sender,
        api_data.receiver,
        Response::Status,
        Command::GetStatus
    ) {
        Ok(status) => HttpResponse::Ok().json(status),
        Err(message) => HttpResponse::BadRequest().json(api_response::Error { message }),
    }
}

#[get("/library")]
async fn library(state: APIDataType) -> HttpResponse {
    let api_data = state.lock().unwrap();

    match send_message!(
        api_data.sender,
        api_data.receiver,
        Response::SoundLibrary,
        Command::GetSoundLibrary
    ) {
        Ok(library) => HttpResponse::Ok().json(library),
        Err(message) => HttpResponse::BadRequest().json(api_response::Error { message }),
    }
}

#[derive(Deserialize)]
struct Volume {
    value: f32,
}

#[post("/volume")]
async fn volume(state: APIDataType, payload: web::Json<Volume>) -> HttpResponse {
    let api_data = state.lock().unwrap();
    match send_message!(
        api_data.sender,
        api_data.receiver,
        Command::SetVolume {
            value: payload.value
        }
    ) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(message) => HttpResponse::BadRequest().json(api_response::Error { message }),
    }
}

#[get("/driver")]
async fn driver(state: APIDataType) -> HttpResponse {
    let api_data = state.lock().unwrap();

    match send_message!(
        api_data.sender,
        api_data.receiver,
        Response::Driver,
        Command::GetDriver
    ) {
        Ok(driver) => HttpResponse::Ok().json(driver),
        Err(message) => HttpResponse::BadRequest().json(api_response::Error { message }),
    }
}

#[get("/driverlist")]
async fn driverlist(state: APIDataType) -> HttpResponse {
    let api_data = state.lock().unwrap();

    match send_message!(
        api_data.sender,
        api_data.receiver,
        Response::DriverList,
        Command::GetDriverList
    ) {
        Ok(driverlist) => HttpResponse::Ok().json(driverlist),
        Err(message) => HttpResponse::BadRequest().json(api_response::Error { message }),
    }
}

#[derive(Deserialize)]
struct Driver {
    id: i32,
}

#[post("/driver")]
async fn set_driver(state: APIDataType, payload: web::Json<Driver>) -> HttpResponse {
    let api_data = state.lock().unwrap();
    match send_message!(
        api_data.sender,
        api_data.receiver,
        Command::SetDriver { id: payload.id }
    ) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(message) => HttpResponse::BadRequest().json(api_response::Error { message }),
    }
}

pub async fn start_web_service(
    host: String,
    port: u32,
    sender: ChannelSender,
    receiver: ResponseReceiver,
    allowed_token: String,
) -> io::Result<()> {
    let data = Arc::new(Mutex::new(APIData::new(sender, receiver)));

    HttpServer::new(move || {
        App::new()
            .data(data.clone())
            .wrap(Logger::default())
            .wrap(TokenAuthorization::new(&allowed_token))
            .service(play)
            .service(pause)
            .service(preview)
            .service(status)
            .service(theme)
            .service(trigger)
            .service(library)
            .service(volume)
            .service(driver)
            .service(driverlist)
            .service(set_driver)
    })
    .bind(format!("{}:{}", host, port))?
    .start()
    .await
}
