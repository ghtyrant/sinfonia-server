use audio_engine::messages::command::LoadTheme;
use std::sync::mpsc::SendError;

quick_error! {
    #[derive(Debug)]
    pub enum ServerError {
        ParseFailed(message: String) {
            description("Failed to parse theme")
            display(r#"Failed to parse theme: {}"#, message)
        }
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum AudioControllerError {
        GenericError {
            description("Unknown AudioController error!")
        }

        CommunicationSendError(e: SendError<LoadTheme>) {
            description("Communication send error")
            display(r#"Communication send error: {}"#, e)
        }
    }
}
