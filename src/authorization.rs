use std::task::{Context, Poll};

use actix_service::{Service, Transform};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{Error, HttpResponse};
use futures::future::{ok, Either, Ready};

pub struct TokenAuthorization {
    token: String,
}

impl TokenAuthorization {
    pub fn new(token: &str) -> Self {
        Self {
            token: token.into(),
        }
    }
}

impl<S, B> Transform<S> for TokenAuthorization
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = TokenAuthorizationMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(TokenAuthorizationMiddleware {
            service,
            token: self.token.clone(),
        })
    }
}
pub struct TokenAuthorizationMiddleware<S> {
    service: S,
    token: String,
}

impl<S, B> Service for TokenAuthorizationMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Either<S::Future, Ready<Result<Self::Response, Self::Error>>>;

    fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let authorization = req.head().headers().get("Authorization");

        match authorization {
            Some(token) => {
                let token_parts: Vec<&str> = token.to_str().unwrap().split(' ').collect();
                if token_parts.len() != 2
                    || token_parts[0] != "Bearer"
                    || token_parts[1] != self.token
                {
                    Either::Right(ok(
                        req.into_response(HttpResponse::Forbidden().finish().into_body())
                    ))
                } else {
                    Either::Left(self.service.call(req))
                }
            }
            None => Either::Right(ok(
                req.into_response(HttpResponse::Forbidden().finish().into_body())
            )),
        }
    }
}
