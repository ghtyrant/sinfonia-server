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

        pub enum Response {
            $($name($name)),*
        }
    }
}

macro_rules! build_response {
    ($name: ident) => {
        response::Response::$name(response::$name {})
    };

    ($name: ident, $( $param_name: ident: $param: expr ),*) => {
        response::Response::$name(response::$name { $( $param_name: $param ),* })
    };
}

pub mod response {
    use std::collections::HashMap;

    responses!(
        Error {
            message: String
        }

        Success {}

        Status {
            playing: bool,
            theme_loaded: bool,
            theme: Option<String>,
            sounds_playing: Vec<String>,
            sounds_playing_next: HashMap<String, u64>,
            previewing: Vec<String>
        }

        LoadTheme {
            success: bool
        }

        Trigger {
            success: bool,
            trigger_found: bool
        }

        SoundLibrary {
            samples: Vec<String>
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
    ($name: ident {
        $($param_name: ident : $param_type: ty),*
    }) => {
        pub struct $name {
            $(pub $param_name: $param_type),*
        }
    }
}

macro_rules! commands {
    ($(
        $name: ident {
            $($param_name: ident : $param_type: ty),*
        }
    )*) => {
        $(__command!($name { $($param_name : $param_type),* });)*

        pub enum Command {
            $($name($name)),*
        }
    }
}

macro_rules! build_command {
    ($name: ident) => {
        command::Command::$name(command::$name{ })
    };

    ($name: ident, $( $param_name: ident: $param: expr ),*) => {
        command::Command::$name(command::$name{ $($param_name: $param),* })
    }
}

pub mod command {
    use theme::Theme;

    commands!(
        Quit {}
        Play {}
        Pause {}
        GetStatus {}
        GetSoundLibrary {}
        GetDriver {}
        GetDriverList {}

        SetDriver {
            id: i32
        }
        Volume {
            value: f32
        }
        PreviewSound {
            sound: String
        }
        LoadTheme {
            theme: Theme
        }
        Trigger {
            sound: String
        }
    );
}
