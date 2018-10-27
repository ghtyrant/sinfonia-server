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

macro_rules! build_command {
    ($name: ident) => {
        command::Command::$name(command::$name{ })
    };

    ($name: ident, $($param: expr),*) => {
        command::Command::$name(command::$name{ $($param),* })
    }
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
