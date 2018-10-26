use rustful::filter::{FilterContext, ResponseAction, ResponseFilter};
use rustful::{Context, DefaultRouter, Headers, Response, Server, StatusCode};

struct JsonResponse;

impl JsonResponse {
    pub fn new(allowed_token: String) -> JsonResponse {
        JsonResponse
    }
}

impl ResponseFilter for JsonResponse {
    fn begin(
        &self,
        ctx: FilterContext,
        status: StatusCode,
        _headers: &mut Headers,
    ) -> (StatusCode, ResponseAction) {
        //Check if a JSONP function is defined and write the beginning of the call
        let output = if let Some(&JsonVar(var)) = ctx.storage.get() {
            Some(format!("{{\"{}\": ", var))
        } else {
            None
        };

        (status, ResponseAction::next(output))
    }

    fn write<'a>(&'a self, _ctx: FilterContext, bytes: Option<Data<'a>>) -> ResponseAction {
        ResponseAction::next(bytes)
    }

    fn end(&self, ctx: FilterContext) -> ResponseAction {
        //Check if a JSONP function is defined and write the end of the call
        let output = ctx.storage.get::<JsonVar>().map(|_| "}");
        ResponseAction::next(output)
    }
}
