use std::io::Result;

use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use futures::future;
use unicase::Ascii;

use gotham;
use gotham::handler::{Handler, HandlerFuture, NewHandler};
use gotham::http::response::create_response;
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::State;

use hyper::header::{AccessControlAllowHeaders, AccessControlAllowOrigin};
use hyper::{Response, StatusCode};

use gotham_serde_json_body_parser::create_json_response;

#[macro_use]
use audio_engine::messages;
use audio_engine::messages::{command, response};
use authorization::AuthorizationTokenMiddleware;

pub type ChannelSender = Sender<command::Command>;
pub type ResponseReceiver = Receiver<response::Response>;

macro_rules! __api_response {
    ($name: ident {
        $($param_name: ident : $param_type: ty),*
    }) => {
        #[derive(Serialize)]
        pub struct $name {
            $(pub $param_name: $param_type),*
        }
    }
}

macro_rules! api_responses {
    ($(
        $name: ident {
            $($param_name: ident : $param_type: ty),*
        }
    )*) => {
        $(__api_response!($name { $($param_name : $param_type),* });)*
    }
}

pub mod api_response {
    api_responses!(
        Error {
            error: bool,
            message: String
        }

        TriggerSound {
            name: String
        }

        PreviewSound {
            name: String
        }

        Volume {
            value: f32
        }

        Driver {
            id: i32
        }
    );
}

#[derive(Clone)]
pub enum SenderHandler {
    Pause {
        sender: Arc<Mutex<ChannelSender>>,
        response_receiver: Arc<Mutex<ResponseReceiver>>,
    },
    /*Play { sender: Arc<Mutex<ChannelSender>> },
    PreviewSound { sender: Arc<Mutex<ChannelSender>> },
    UploadTheme { sender: Arc<Mutex<ChannelSender>> },
    Trigger { sender: Arc<Mutex<ChannelSender>> },
    GetStatus { sender: Arc<Mutex<ChannelSender>> },
    GetSoundLibrary { sender: Arc<Mutex<ChannelSender>> },
    Volume { sender: Arc<Mutex<ChannelSender>> },
    GetDriverList { sender: Arc<Mutex<ChannelSender>> },
    GetDriver { sender: Arc<Mutex<ChannelSender>> },
    SetDriver { sender: Arc<Mutex<ChannelSender>> },*/
}

fn add_cors_headers(res: &mut Response) {
    let headers = res.headers_mut();
    headers.set(AccessControlAllowOrigin::Any);
    headers.set(AccessControlAllowHeaders(vec![
        Ascii::new("authorization".to_owned()),
        Ascii::new("content-type".to_owned()),
    ]));
}

macro_rules! send_message {
    ($sender: ident, $receiver: ident, $response: ident, $message: expr) => {{
        $sender
            .lock()
            .unwrap()
            .send($message)
            .expect("Failed to communicate with audio engine!");

        match $receiver.lock().unwrap().recv().expect("Failed to communicate with audio engine!") {
            response::Response::$response(response) => Ok(response),
            response::Response::Error(response) => Err(response),
            _ => panic!("Internal Error!"),
        }
    }};

    ($sender: ident, $receiver: ident, $message: expr) => {{
        send_message!($sender, $receiver, Success, $message)
    }};
}

impl Handler for SenderHandler {
    fn handle(self, mut state: State) -> Box<HandlerFuture> {
        match self {
            SenderHandler::Pause {
                ref sender,
                ref response_receiver,
            } => {
                let res = match send_message!(
                    sender,
                    response_receiver,
                    build_command!(Pause)
                ) {
                    Ok(_) => create_response(&state, StatusCode::Ok, None),
                    Err(error) => {
                        error!("Error in Pause: {}", &error.message);

                        let mut res = create_json_response(
                            &state,
                            StatusCode::BadRequest,
                            &api_response::Error {
                                error: true,
                                message: error.message,
                            },
                        ).unwrap();
                        add_cors_headers(&mut res);

                        res
                    }
                };

                Box::new(future::ok((state, res)))
            } /*SenderHandler::Play { ref sender } => {
                sender
                    .lock()
                    .unwrap()
                    .send(AudioControllerMessage::Play {})
                    .expect("Failed to send AudioControllerMessage::Play!");

                let mut res = create_json_response(
                    &state,
                    StatusCode::Ok,
                    &ApiResponse {
                        success: true,
                        message: "Hello World!".into(),
                    },
                ).unwrap();
                add_cors_headers(&mut res);
                Box::new(future::ok((state, res)))
            }

            SenderHandler::PreviewSound { sender } => {
                let body = Body::take_from(&mut state);
                let parsing = body.concat2().map_err(Error::from).then(move |body| {
                    let text = match body {
                        Ok(text) => text,
                        Err(_) => {
                            let mut res =
                                create_response(&state, StatusCode::InternalServerError, None);
                            add_cors_headers(&mut res);
                            return Ok((state, res));
                        }
                    };

                    let json = String::from_utf8(text.to_vec()).unwrap();
                    let sound = match serde_json::from_str::<PreviewSound>(&json) {
                        Ok(sound) => sound,
                        Err(e) => {
                            error!("Failed to parse play sound request: {}", e);
                            let mut res = create_json_response(
                                &state,
                                StatusCode::UnprocessableEntity,
                                &ApiResponse {
                                    success: true,
                                    message: format!("{}", e),
                                },
                            ).unwrap();
                            add_cors_headers(&mut res);
                            return Ok((state, res));
                        }
                    };

                    sender
                        .lock()
                        .unwrap()
                        .send(AudioControllerMessage::PreviewSound { sound: sound.name })
                        .expect("Failed to send AudioControllerMessage::PreviewSound!");

                    let mut res = create_response(&state, StatusCode::Ok, None);
                    add_cors_headers(&mut res);
                    Ok((state, res))
                });

                Box::new(parsing)
            }

            // XXX This is some ugly ass code below
            SenderHandler::UploadTheme { sender } => {
                let body = Body::take_from(&mut state);
                let theme_parsing = body.concat2().map_err(Error::from).then(move |body| {
                    let text = match body {
                        Ok(text) => text,
                        Err(_) => {
                            let mut res =
                                create_response(&state, StatusCode::InternalServerError, None);
                            add_cors_headers(&mut res);
                            return Ok((state, res));
                        }
                    };

                    let json = String::from_utf8(text.to_vec()).unwrap();
                    let theme = match serde_json::from_str::<Theme>(&json) {
                        Ok(theme) => theme,
                        Err(e) => {
                            error!("Failed to parse theme: {}", e);
                            let mut res = create_json_response(
                                &state,
                                StatusCode::UnprocessableEntity,
                                &ApiResponse {
                                    success: true,
                                    message: format!("{}", e),
                                },
                            ).unwrap();
                            add_cors_headers(&mut res);
                            return Ok((state, res));
                        }
                    };

                    let (response_sender, response_receiver) = channel();
                    sender
                        .lock()
                        .unwrap()
                        .send(AudioControllerMessage::LoadTheme {
                            theme,
                            response_sender,
                        }).expect("Failed to send AudioControllerMessage::UploadTheme!");

                    let response = response_receiver.recv().unwrap();
                    let mut res = create_json_response(
                        &state,
                        StatusCode::Ok,
                        &ApiResponse {
                            success: response.success,
                            message: "".into(),
                        },
                    ).unwrap();
                    add_cors_headers(&mut res);
                    Ok((state, res))
                });

                Box::new(theme_parsing)
            }

            SenderHandler::Trigger { sender } => Box::new(state.json::<TriggerSound>().and_then(
                move |(state, trigger)| {
                    let (response_sender, response_receiver) = channel();
                    sender
                        .lock()
                        .unwrap()
                        .send(AudioControllerMessage::Trigger {
                            sound: trigger.name,
                            response_sender,
                        }).expect("Failed to send AudioControllerMessage::Trigger!");

                    let response = response_receiver.recv().unwrap();
                    let status = if response.trigger_found {
                        StatusCode::Ok
                    } else {
                        StatusCode::NotFound
                    };

                    let mut res = create_json_response(
                        &state,
                        status,
                        &ApiResponse {
                            success: response.trigger_found,
                            message: "Hello World!".into(),
                        },
                    ).unwrap();
                    add_cors_headers(&mut res);
                    Ok((state, res))
                },
            )),

            SenderHandler::GetStatus { sender } => {
                let (response_sender, response_receiver) = channel();
                sender
                    .lock()
                    .unwrap()
                    .send(AudioControllerMessage::GetStatus { response_sender })
                    .expect("Failed to send AudioControllerMessage::GetStatus!");

                let response = response_receiver.recv().unwrap();
                let mut res = create_json_response(&state, StatusCode::Ok, &response).unwrap();
                add_cors_headers(&mut res);
                Box::new(future::ok((state, res)))
            }

            SenderHandler::GetSoundLibrary { sender } => {
                let (response_sender, response_receiver) = channel();
                sender
                    .lock()
                    .unwrap()
                    .send(AudioControllerMessage::GetSoundLibrary { response_sender })
                    .expect("Failed to send AudioControllerMessage::GetSoundLibrary!");

                let response = response_receiver.recv().unwrap();
                let mut res = create_json_response(&state, StatusCode::Ok, &response).unwrap();
                add_cors_headers(&mut res);
                Box::new(future::ok((state, res)))
            }

            SenderHandler::Volume { sender } => {
                Box::new(state.json::<Volume>().and_then(move |(state, volume)| {
                    sender
                        .lock()
                        .unwrap()
                        .send(AudioControllerMessage::Volume {
                            value: volume.value,
                        }).expect("Failed to send AudioControllerMessage::Volume!");

                    let mut res = create_json_response(
                        &state,
                        StatusCode::Ok,
                        &ApiResponse {
                            success: true,
                            message: "Hello World!".into(),
                        },
                    ).unwrap();
                    add_cors_headers(&mut res);
                    Ok((state, res))
                }))
            }

            SenderHandler::GetDriver { sender } => {
                let (response_sender, response_receiver) = channel();

                sender
                    .lock()
                    .unwrap()
                    .send(AudioControllerMessage::GetDriver { response_sender })
                    .expect("Failed to send AudioControllerMessage::GetDriver!");

                let response = response_receiver.recv().unwrap();
                let mut res = create_json_response(&state, StatusCode::Ok, &response).unwrap();
                add_cors_headers(&mut res);
                Box::new(future::ok((state, res)))
            }

            SenderHandler::GetDriverList { sender } => {
                let (response_sender, response_receiver) = channel();

                sender
                    .lock()
                    .unwrap()
                    .send(AudioControllerMessage::GetDriverList { response_sender })
                    .expect("Failed to send AudioControllerMessage::GetDriverList!");

                let response = response_receiver.recv().unwrap();
                let mut res = create_json_response(&state, StatusCode::Ok, &response).unwrap();
                add_cors_headers(&mut res);
                Box::new(future::ok((state, res)))
            }

            SenderHandler::SetDriver { sender } => {
                Box::new(state.json::<Driver>().and_then(move |(state, driver)| {
                    sender
                        .lock()
                        .unwrap()
                        .send(AudioControllerMessage::SetDriver { id: driver.id })
                        .expect("Failed to send AudioControllerMessage::SetDriver!");

                    let mut res = create_json_response(
                        &state,
                        StatusCode::Ok,
                        &ApiResponse {
                            success: true,
                            message: "Hello World!".into(),
                        },
                    ).unwrap();
                    add_cors_headers(&mut res);
                    Ok((state, res))
                }))
            }*/
        }
    }
}

impl NewHandler for SenderHandler {
    type Instance = Self;

    fn new_handler(&self) -> Result<Self::Instance> {
        Ok(self.clone())
    }
}

fn cors_allow_all(state: State) -> (State, Response) {
    let mut res = create_response(&state, StatusCode::Ok, None);

    add_cors_headers(&mut res);

    (state, res)
}

fn router(
    sender: &ChannelSender,
    response_receiver: &Arc<Mutex<ResponseReceiver>>,
    allowed_token: String,
) -> Router {
    let (chain, pipeline) = single_pipeline(
        new_pipeline()
            .add(AuthorizationTokenMiddleware::new(allowed_token))
            .build(),
    );

    build_router(chain, pipeline, |route| {
        route.post("/pause").to_new_handler(SenderHandler::Pause {
            sender: Arc::new(Mutex::new(sender.clone())),
            response_receiver: response_receiver.clone(),
        });
        /*route.post("/play").to_new_handler(SenderHandler::Play {
            sender: Arc::new(Mutex::new(sender.clone())),
        });
        route
            .post("/preview")
            .to_new_handler(SenderHandler::PreviewSound {
                sender: Arc::new(Mutex::new(sender.clone())),
            });
        route
            .post("/trigger")
            .to_new_handler(SenderHandler::Trigger {
                sender: Arc::new(Mutex::new(sender.clone())),
            });
        route
            .post("/theme")
            .to_new_handler(SenderHandler::UploadTheme {
                sender: Arc::new(Mutex::new(sender.clone())),
            });
        route
            .get("/status")
            .to_new_handler(SenderHandler::GetStatus {
                sender: Arc::new(Mutex::new(sender.clone())),
            });
        route
            .get("/library")
            .to_new_handler(SenderHandler::GetSoundLibrary {
                sender: Arc::new(Mutex::new(sender.clone())),
            });
        route.post("/volume").to_new_handler(SenderHandler::Volume {
            sender: Arc::new(Mutex::new(sender.clone())),
        });
        route
            .get("/driver/list")
            .to_new_handler(SenderHandler::GetDriverList {
                sender: Arc::new(Mutex::new(sender.clone())),
            });
        route
            .get("/driver")
            .to_new_handler(SenderHandler::GetDriver {
                sender: Arc::new(Mutex::new(sender.clone())),
            });
        route
            .post("/driver")
            .to_new_handler(SenderHandler::SetDriver {
                sender: Arc::new(Mutex::new(sender.clone())),
            });*/

        route.options("/play").to(cors_allow_all);
        route.options("/preview").to(cors_allow_all);
        route.options("/pause").to(cors_allow_all);
        route.options("/trigger").to(cors_allow_all);
        route.options("/theme").to(cors_allow_all);
        route.options("/status").to(cors_allow_all);
        route.options("/library").to(cors_allow_all);
        route.options("/volume").to(cors_allow_all);
        route.options("/driver").to(cors_allow_all);
        route.options("/driver/list").to(cors_allow_all);
    })
}

pub fn start_web_service(
    address: String,
    threads: usize,
    sender: &ChannelSender,
    response_receiver: ResponseReceiver,
    allowed_token: String,
) {
    info!("Starting up API ...");
    gotham::start_with_num_threads(
        address,
        threads,
        router(
            &sender,
            &Arc::new(Mutex::new(response_receiver)),
            allowed_token,
        ),
    );
}
