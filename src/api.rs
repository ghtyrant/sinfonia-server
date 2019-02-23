use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use failure::Error;

use futures::{future, Future, Stream};
use unicase::Ascii;

use gotham;
use gotham::handler::{Handler, HandlerFuture, NewHandler};
use gotham::http::response::create_response;
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State};

use hyper::header::{AccessControlAllowHeaders, AccessControlAllowOrigin};
use hyper::{Body, Response, StatusCode};

use gotham_serde_json_body_parser::create_json_response;
use serde::de::DeserializeOwned;

use audio_engine::messages::{command, response};
use authorization::AuthorizationTokenMiddleware;
use theme::Theme;

pub type ChannelSender = Sender<command::Command>;
pub type ResponseReceiver = Receiver<response::Response>;

macro_rules! __api_parameter {
    ($name: ident {
        $($param_name: ident : $param_type: ty),*
    }) => {
        #[derive(Deserialize)]
        pub struct $name {
            $(pub $param_name: $param_type),*
        }
    }
}

macro_rules! api_parameters {
    ($(
        $name: ident {
            $($param_name: ident : $param_type: ty),*
        }
    )*) => {
        $(__api_parameter!($name { $($param_name : $param_type),* });)*
    }
}

pub mod api_parameter {
    api_parameters!(
        Trigger {
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

pub mod api_response {
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
    }

    #[derive(Serialize)]
    pub struct SoundLibrary {
        pub samples: Vec<String>,
    }
}

#[derive(Clone)]
pub enum SenderHandler {
    Pause {
        sender: Arc<Mutex<ChannelSender>>,
        response_receiver: Arc<Mutex<ResponseReceiver>>,
    },
    Play {
        sender: Arc<Mutex<ChannelSender>>,
        response_receiver: Arc<Mutex<ResponseReceiver>>,
    },
    PreviewSound {
        sender: Arc<Mutex<ChannelSender>>,
        response_receiver: Arc<Mutex<ResponseReceiver>>,
    },
    UploadSounds {
        sender: Arc<Mutex<ChannelSender>>,
        response_receiver: Arc<Mutex<ResponseReceiver>>,
    },
    Trigger {
        sender: Arc<Mutex<ChannelSender>>,
        response_receiver: Arc<Mutex<ResponseReceiver>>,
    },
    GetStatus {
        sender: Arc<Mutex<ChannelSender>>,
        response_receiver: Arc<Mutex<ResponseReceiver>>,
    },
    GetSoundLibrary {
        sender: Arc<Mutex<ChannelSender>>,
        response_receiver: Arc<Mutex<ResponseReceiver>>,
    },
    Volume {
        sender: Arc<Mutex<ChannelSender>>,
        response_receiver: Arc<Mutex<ResponseReceiver>>,
    },
    GetDriverList {
        sender: Arc<Mutex<ChannelSender>>,
        response_receiver: Arc<Mutex<ResponseReceiver>>,
    },
    GetDriver {
        sender: Arc<Mutex<ChannelSender>>,
        response_receiver: Arc<Mutex<ResponseReceiver>>,
    },
    SetDriver {
        sender: Arc<Mutex<ChannelSender>>,
        response_receiver: Arc<Mutex<ResponseReceiver>>,
    },
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
            .expect("Failed to lock sender mutex!")
            .send($message)
            .expect("Failed to communicate with audio engine!");

        match $receiver
            .lock()
            .expect("Failed to lock receiver mutex!")
            .recv()
            .expect("Failed to communicate with audio engine!")
        {
            response::Response::$response(response) => Ok(response),
            response::Response::Error(response) => Err(response),
            _ => panic!("Internal Error!"),
        }
    }};

    ($sender: ident, $receiver: ident, $message: expr) => {{
        send_message!($sender, $receiver, Success, $message)
    }};
}

fn parse_parameter<T>(
    state: &State,
    body: Result<hyper::Chunk, failure::Error>,
) -> Result<T, Response>
where
    T: DeserializeOwned,
{
    let text = match body {
        Ok(text) => text,
        Err(_) => {
            let mut res = create_response(state, StatusCode::InternalServerError, None);
            add_cors_headers(&mut res);
            return Err(res);
        }
    };

    let json = String::from_utf8(text.to_vec()).unwrap();
    let object = match serde_json::from_str::<T>(&json) {
        Ok(sound) => sound,
        Err(e) => {
            error!("Failed to parse play sound request: {}", e);
            let mut res = create_json_response(
                &state,
                StatusCode::UnprocessableEntity,
                &api_response::Error {
                    message: format!("{}", e),
                },
            )
            .unwrap();
            add_cors_headers(&mut res);
            return Err(res);
        }
    };

    Ok(object)
}

impl Handler for SenderHandler {
    fn handle(self, mut state: State) -> Box<HandlerFuture> {
        match self {
            SenderHandler::Pause {
                ref sender,
                ref response_receiver,
            } => {
                let res = match send_message!(sender, response_receiver, build_command!(Pause)) {
                    Ok(_) => create_response(&state, StatusCode::Ok, None),
                    Err(error) => {
                        error!("Error in Pause: {}", &error.message);

                        let mut res = create_json_response(
                            &state,
                            StatusCode::BadRequest,
                            &api_response::Error {
                                message: error.message,
                            },
                        )
                        .unwrap();
                        add_cors_headers(&mut res);

                        res
                    }
                };

                Box::new(future::ok((state, res)))
            }

            SenderHandler::Play {
                ref sender,
                ref response_receiver,
            } => {
                let res = match send_message!(sender, response_receiver, build_command!(Play)) {
                    Ok(_) => create_response(&state, StatusCode::Ok, None),
                    Err(error) => {
                        error!("Error in Pause: {}", &error.message);

                        let mut res = create_json_response(
                            &state,
                            StatusCode::BadRequest,
                            &api_response::Error {
                                message: error.message,
                            },
                        )
                        .unwrap();
                        add_cors_headers(&mut res);

                        res
                    }
                };

                Box::new(future::ok((state, res)))
            }

            SenderHandler::PreviewSound {
                sender,
                response_receiver,
            } => {
                let body = Body::take_from(&mut state);
                let parsing = body.concat2().map_err(Error::from).then(move |body| {
                    let sound = match parse_parameter::<api_parameter::PreviewSound>(&state, body) {
                        Ok(sound) => sound,
                        Err(res) => return Box::new(future::ok((state, res))),
                    };

                    let res = match send_message!(
                        sender,
                        response_receiver,
                        build_command!(PreviewSound, sound: sound.name)
                    ) {
                        Ok(_) => create_response(&state, StatusCode::Ok, None),
                        Err(error) => {
                            error!("PreviewSound: {}", &error.message);

                            let mut res = create_json_response(
                                &state,
                                StatusCode::BadRequest,
                                &api_response::Error {
                                    message: error.message,
                                },
                            )
                            .unwrap();
                            add_cors_headers(&mut res);

                            res
                        }
                    };

                    Box::new(future::ok((state, res)))
                });

                Box::new(parsing)
            }

            SenderHandler::UploadSounds {
                sender,
                response_receiver,
            } => {
                let body = Body::take_from(&mut state);
                let parsing = body.concat2().map_err(Error::from).then(move |body| {
                    let theme = match parse_parameter::<Theme>(&state, body) {
                        Ok(t) => t,
                        Err(res) => return Box::new(future::ok((state, res))),
                    };

                    let res = match send_message!(
                        sender,
                        response_receiver,
                        build_command!(LoadTheme, theme: theme)
                    ) {
                        Ok(_) => create_response(&state, StatusCode::Ok, None),
                        Err(error) => {
                            error!("LoadTheme: {}", &error.message);

                            let mut res = create_json_response(
                                &state,
                                StatusCode::BadRequest,
                                &api_response::Error {
                                    message: error.message,
                                },
                            )
                            .unwrap();
                            add_cors_headers(&mut res);

                            res
                        }
                    };

                    Box::new(future::ok((state, res)))
                });

                Box::new(parsing)
            }

            SenderHandler::Trigger {
                sender,
                response_receiver,
            } => {
                let body = Body::take_from(&mut state);
                let parsing = body.concat2().map_err(Error::from).then(move |body| {
                    let trigger = match parse_parameter::<api_parameter::Trigger>(&state, body) {
                        Ok(t) => t,
                        Err(res) => return Box::new(future::ok((state, res))),
                    };

                    let res = match send_message!(
                        sender,
                        response_receiver,
                        build_command!(Trigger, sound: trigger.name)
                    ) {
                        Ok(_) => create_response(&state, StatusCode::Ok, None),
                        Err(error) => {
                            error!("Trigger: {}", &error.message);

                            let mut res = create_json_response(
                                &state,
                                StatusCode::NotFound,
                                &api_response::Error {
                                    message: error.message,
                                },
                            )
                            .unwrap();
                            add_cors_headers(&mut res);

                            res
                        }
                    };

                    Box::new(future::ok((state, res)))
                });

                Box::new(parsing)
            }

            SenderHandler::GetStatus {
                ref sender,
                ref response_receiver,
            } => {
                let res = match send_message!(
                    sender,
                    response_receiver,
                    Status,
                    build_command!(GetStatus)
                ) {
                    Ok(status) => create_json_response(
                        &state,
                        StatusCode::Ok,
                        &api_response::Status {
                            playing: status.playing,
                            theme_loaded: status.theme_loaded,
                            theme: status.theme,
                            sounds_playing: status.sounds_playing,
                        },
                    )
                    .unwrap(),
                    Err(error) => {
                        error!("GetStatus: {}", &error.message);

                        let mut res = create_json_response(
                            &state,
                            StatusCode::NotFound,
                            &api_response::Error {
                                message: error.message,
                            },
                        )
                        .unwrap();
                        add_cors_headers(&mut res);

                        res
                    }
                };

                Box::new(future::ok((state, res)))
            }

            SenderHandler::GetSoundLibrary {
                ref sender,
                ref response_receiver,
            } => {
                let res = match send_message!(
                    sender,
                    response_receiver,
                    SoundLibrary,
                    build_command!(GetSoundLibrary)
                ) {
                    Ok(library) => create_json_response(
                        &state,
                        StatusCode::Ok,
                        &api_response::SoundLibrary {
                            samples: library.samples,
                        },
                    )
                    .unwrap(),
                    Err(error) => {
                        error!("GetSoundLibrary: {}", &error.message);

                        let mut res = create_json_response(
                            &state,
                            StatusCode::NotFound,
                            &api_response::Error {
                                message: error.message,
                            },
                        )
                        .unwrap();
                        add_cors_headers(&mut res);

                        res
                    }
                };

                Box::new(future::ok((state, res)))
            }

            SenderHandler::Volume {
                sender,
                response_receiver,
            } => {
                let body = Body::take_from(&mut state);
                let parsing = body.concat2().map_err(Error::from).then(move |body| {
                    let volume = match parse_parameter::<api_parameter::Volume>(&state, body) {
                        Ok(t) => t,
                        Err(res) => return Box::new(future::ok((state, res))),
                    };

                    let res = match send_message!(
                        sender,
                        response_receiver,
                        build_command!(Volume, value: volume.value)
                    ) {
                        Ok(_) => create_response(&state, StatusCode::Ok, None),
                        Err(error) => {
                            error!("Volume: {}", &error.message);

                            let mut res = create_json_response(
                                &state,
                                StatusCode::NotFound,
                                &api_response::Error {
                                    message: error.message,
                                },
                            )
                            .unwrap();
                            add_cors_headers(&mut res);

                            res
                        }
                    };

                    Box::new(future::ok((state, res)))
                });

                Box::new(parsing)
            }

            SenderHandler::GetDriver {
                ref sender,
                ref response_receiver,
            } => {
                let res = match send_message!(sender, response_receiver, build_command!(GetDriver))
                {
                    Ok(_) => create_response(&state, StatusCode::Ok, None),
                    Err(error) => {
                        error!("GetSoundLibrary: {}", &error.message);

                        let mut res = create_json_response(
                            &state,
                            StatusCode::NotFound,
                            &api_response::Error {
                                message: error.message,
                            },
                        )
                        .unwrap();
                        add_cors_headers(&mut res);

                        res
                    }
                };

                Box::new(future::ok((state, res)))
            }
            SenderHandler::GetDriverList {
                ref sender,
                ref response_receiver,
            } => {
                let res =
                    match send_message!(sender, response_receiver, build_command!(GetDriverList)) {
                        Ok(_) => create_response(&state, StatusCode::Ok, None),
                        Err(error) => {
                            error!("GetSoundLibrary: {}", &error.message);

                            let mut res = create_json_response(
                                &state,
                                StatusCode::NotFound,
                                &api_response::Error {
                                    message: error.message,
                                },
                            )
                            .unwrap();
                            add_cors_headers(&mut res);

                            res
                        }
                    };

                Box::new(future::ok((state, res)))
            }
            SenderHandler::SetDriver {
                sender,
                response_receiver,
            } => {
                let body = Body::take_from(&mut state);
                let parsing = body.concat2().map_err(Error::from).then(move |body| {
                    let driver = match parse_parameter::<api_parameter::Driver>(&state, body) {
                        Ok(t) => t,
                        Err(res) => return Box::new(future::ok((state, res))),
                    };

                    let res = match send_message!(
                        sender,
                        response_receiver,
                        build_command!(SetDriver, id: driver.id)
                    ) {
                        Ok(_) => create_response(&state, StatusCode::Ok, None),
                        Err(error) => {
                            error!("Volume: {}", &error.message);

                            let mut res = create_json_response(
                                &state,
                                StatusCode::NotFound,
                                &api_response::Error {
                                    message: error.message,
                                },
                            )
                            .unwrap();
                            add_cors_headers(&mut res);

                            res
                        }
                    };

                    Box::new(future::ok((state, res)))
                });

                Box::new(parsing)
            }
        }
    }
}

impl NewHandler for SenderHandler {
    type Instance = Self;

    fn new_handler(&self) -> std::io::Result<Self::Instance> {
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
        route.post("/play").to_new_handler(SenderHandler::Play {
            sender: Arc::new(Mutex::new(sender.clone())),
            response_receiver: response_receiver.clone(),
        });
        route
            .post("/preview")
            .to_new_handler(SenderHandler::PreviewSound {
                sender: Arc::new(Mutex::new(sender.clone())),
                response_receiver: response_receiver.clone(),
            });
        route
            .post("/trigger")
            .to_new_handler(SenderHandler::Trigger {
                sender: Arc::new(Mutex::new(sender.clone())),
                response_receiver: response_receiver.clone(),
            });
        route
            .post("/sounds")
            .to_new_handler(SenderHandler::UploadSounds {
                sender: Arc::new(Mutex::new(sender.clone())),
                response_receiver: response_receiver.clone(),
            });
        route
            .get("/status")
            .to_new_handler(SenderHandler::GetStatus {
                sender: Arc::new(Mutex::new(sender.clone())),
                response_receiver: response_receiver.clone(),
            });
        route
            .get("/library")
            .to_new_handler(SenderHandler::GetSoundLibrary {
                sender: Arc::new(Mutex::new(sender.clone())),
                response_receiver: response_receiver.clone(),
            });
        route.post("/volume").to_new_handler(SenderHandler::Volume {
            sender: Arc::new(Mutex::new(sender.clone())),
            response_receiver: response_receiver.clone(),
        });
        route
            .get("/driver/list")
            .to_new_handler(SenderHandler::GetDriverList {
                sender: Arc::new(Mutex::new(sender.clone())),
                response_receiver: response_receiver.clone(),
            });
        route
            .get("/driver")
            .to_new_handler(SenderHandler::GetDriver {
                sender: Arc::new(Mutex::new(sender.clone())),
                response_receiver: response_receiver.clone(),
            });
        route
            .post("/driver")
            .to_new_handler(SenderHandler::SetDriver {
                sender: Arc::new(Mutex::new(sender.clone())),
                response_receiver: response_receiver.clone(),
            });

        route.options("/play").to(cors_allow_all);
        route.options("/preview").to(cors_allow_all);
        route.options("/pause").to(cors_allow_all);
        route.options("/trigger").to(cors_allow_all);
        route.options("/sounds").to(cors_allow_all);
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
