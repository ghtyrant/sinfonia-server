use std::io;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{get, http, post, web, App, HttpResponse, HttpServer};

use crate::audio_engine::messages::{command, response};
use crate::authorization::TokenAuthorization;
use crate::theme::Theme;

pub type ChannelSender = Sender<command::Command>;
pub type ResponseReceiver = Receiver<response::Response>;

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
    ($sender: expr, $receiver: expr, $response: ident, $message: expr) => {{
        $sender
            .send($message)
            .expect("Failed to communicate with audio engine!");

        match $receiver
            .recv()
            .expect("Failed to communicate with audio engine!")
        {
            response::Response::$response(response) => Ok(response),
            response::Response::Error(response) => Err(response),
            _ => panic!("Internal Error!"),
        }
    }};

    ($sender: expr, $receiver: expr, $message: expr) => {{
        send_message!($sender, $receiver, Success, $message)
    }};
}

#[post("/pause")]
async fn pause(state: APIDataType) -> HttpResponse {
    let api_data = state.lock().unwrap();

    match send_message!(api_data.sender, api_data.receiver, build_command!(Pause)) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(error) => HttpResponse::BadRequest().json(api_response::Error {
            message: error.message,
        }),
    }
}

#[post("/play")]
async fn play(state: APIDataType) -> HttpResponse {
    let api_data = state.lock().unwrap();

    match send_message!(api_data.sender, api_data.receiver, build_command!(Play)) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(error) => HttpResponse::BadRequest().json(api_response::Error {
            message: error.message,
        }),
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
        build_command!(PreviewSound, sound: payload.name.clone())
    ) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(error) => HttpResponse::BadRequest().json(api_response::Error {
            message: error.message,
        }),
    }
}

#[post("/theme")]
async fn theme(state: APIDataType, payload: web::Json<Theme>) -> HttpResponse {
    let api_data = state.lock().unwrap();
    match send_message!(
        api_data.sender,
        api_data.receiver,
        build_command!(LoadTheme, theme: payload.into_inner())
    ) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(error) => HttpResponse::BadRequest().json(api_response::Error {
            message: error.message,
        }),
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
        build_command!(Trigger, sound: payload.name.clone())
    ) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(error) => HttpResponse::BadRequest().json(api_response::Error {
            message: error.message,
        }),
    }
}

#[get("/status")]
async fn status(state: APIDataType) -> HttpResponse {
    let api_data = state.lock().unwrap();

    match send_message!(
        api_data.sender,
        api_data.receiver,
        Status,
        build_command!(GetStatus)
    ) {
        Ok(status) => HttpResponse::Ok().json(status),
        Err(error) => HttpResponse::BadRequest().json(api_response::Error {
            message: error.message,
        }),
    }
}

#[get("/library")]
async fn library(state: APIDataType) -> HttpResponse {
    let api_data = state.lock().unwrap();

    match send_message!(
        api_data.sender,
        api_data.receiver,
        SoundLibrary,
        build_command!(GetSoundLibrary)
    ) {
        Ok(library) => HttpResponse::Ok().json(library),
        Err(error) => HttpResponse::BadRequest().json(api_response::Error {
            message: error.message,
        }),
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
        build_command!(SetVolume, value: payload.value)
    ) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(error) => HttpResponse::BadRequest().json(api_response::Error {
            message: error.message,
        }),
    }
}

#[get("/driver")]
async fn driver(state: APIDataType) -> HttpResponse {
    let api_data = state.lock().unwrap();

    match send_message!(
        api_data.sender,
        api_data.receiver,
        Driver,
        build_command!(GetDriver)
    ) {
        Ok(driver) => HttpResponse::Ok().json(driver),
        Err(error) => HttpResponse::BadRequest().json(api_response::Error {
            message: error.message,
        }),
    }
}

#[get("/driverlist")]
async fn driverlist(state: APIDataType) -> HttpResponse {
    let api_data = state.lock().unwrap();

    match send_message!(
        api_data.sender,
        api_data.receiver,
        DriverList,
        build_command!(GetDriverList)
    ) {
        Ok(driverlist) => HttpResponse::Ok().json(driverlist),
        Err(error) => HttpResponse::BadRequest().json(api_response::Error {
            message: error.message,
        }),
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
        build_command!(SetDriver, id: payload.id)
    ) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(error) => HttpResponse::BadRequest().json(api_response::Error {
            message: error.message,
        }),
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
            /*.wrap(
                Cors::new()
                    .allowed_origin("All")
                    .send_wildcard()
                    .allowed_methods(vec!["GET", "POST"])
                    .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
                    .allowed_header(http::header::CONTENT_TYPE)
                    .max_age(3600)
                    .finish(),
            )*/
            .service(play)
            .service(pause)
            .service(preview)
            .service(theme)
            .service(trigger)
            .service(status)
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
