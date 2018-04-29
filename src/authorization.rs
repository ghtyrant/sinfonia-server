use std::io::Result;

use gotham::middleware::{NewMiddleware, Middleware};
use gotham::state::{FromState, State};
use gotham::handler::HandlerFuture;
use gotham::http::response::create_response;

use futures::future;

use hyper::{StatusCode, Method};
use hyper::header::{Headers, Authorization, Bearer};

#[derive(Clone)]
pub struct AuthorizationTokenMiddleware {
    allowed_token: String,
}

impl AuthorizationTokenMiddleware {
    pub fn new(token: String) -> Self {
        Self {
            allowed_token: token
        }
    }
}

impl NewMiddleware for AuthorizationTokenMiddleware {
    type Instance = Self;

    fn new_middleware(&self) -> Result<Self::Instance> {
        Ok(self.clone())
    }
}

impl Middleware for AuthorizationTokenMiddleware {
    fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
        where Chain: FnOnce(State) -> Box<HandlerFuture> + 'static
    {
        // Always allow OPTIONS requests
        if *Method::borrow_from(&state) == Method::Options {
            return chain(state)
        }

        let authorized = match Headers::borrow_from(&state).get::<Authorization<Bearer>>() {
            Some(bearer) => {
                if bearer.token == self.allowed_token {
                    true
                }
                else {
                    warn!("Access using a wrong token!");
                    false
                }
            }

            None => {
                warn!("Access without specifying a token, blocked!");
                false
            }
        };

        if !authorized {
            let response = create_response(&state, StatusCode::Unauthorized, None);
            Box::new(future::ok((state, response)))
        } else {
            chain(state)
        }
    }
}