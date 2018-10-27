use std::sync::mpsc::Sender;

macro_rules! __response {
    ($name: ident { 
        $($param_name: ident : $param_type: ty),*
    }) => {
        pub struct $name {
            $(pub $param_name: $param_type),*
        }
    }
}

macro_rules! responses {
    ($(
        $name: ident { 
            $($param_name: ident : $param_type: ty),*
        }
    )*) => {
        $(__response!($name { $($param_name : $param_type),* });)*
    }
}

pub mod response {
    responses!(
        Generic { 
            success: bool
        }
        
        Status {
            playing: bool,
            theme_loaded: bool,
            sounds_playing: Vec<String>
        }

        LoadTheme {
            success: bool
        }

        Trigger {
            success: bool,
            trigger_found: bool
        }

        SoundLibrary {
            sounds: Vec<String>
        }

        DriverList {
            drivers: Vec<(i32, String)>
        }

        Driver {
            id: i32
        }
    );
}

macro_rules! __command {
    ($name: ident -> $response: path { 
        $($param_name: ident : $param_type: ty),*
    }) => {
        pub struct $name {
            $(pub $param_name: $param_type),*
        }

        impl $name {
            pub fn init($($param_name: $param_type),*) -> $name {
                $name {
                    $($param_name),*
                }
            }

            pub fn wrap(&mut self, response_sender: Sender<$response>) -> Command::$name {
                Command::$name(response_sender, self)
            }
        }
    }
}

macro_rules! commands {
    ($(
        $name: ident -> $response: path { 
            $($param_name: ident : $param_type: ty),*
        }
    )*) => {
        $(__command!($name -> $response { $($param_name : $param_type),* });)*

        pub enum Command {
            $($name(Option<Sender<$response>>, $name)),*
        }
    }
}

pub mod command {
    use std::sync::mpsc::Sender;
    use audio_engine::messages::response;

    use theme::Theme;

    commands!(
        Quit -> response::Generic {}
        Play -> response::Generic {}
        Pause -> response::Generic {}
        GetStatus -> response::Status {}
        GetSoundLibrary -> response::SoundLibrary {}
        GetDriver -> response::Driver {}
        GetDriverList -> response::DriverList {}

        SetDriver -> response::Generic {
            id: i32
        }
        Volume -> response::Generic {
            value: f32
        }
        PreviewSound -> response::Generic {
            sound: String
        }
        LoadTheme -> response::LoadTheme {
            theme: Theme
        }
        Trigger -> response::Trigger {
            sound: String
        }
    );
}