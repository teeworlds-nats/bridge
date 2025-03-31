use log::error;
use std::error::Error;
use std::process::exit;

pub fn err_to_string_and_exit(msg: &str, err: Box<dyn Error>) {
    let text = match err.to_string().as_ref() {
        "Broken pipe (os error 32)" => "Server closed socket(Broken pipe, os error 32)".to_string(),
        _ => err.to_string(),
    };
    error!("{}{}", msg, text);
    exit(1);
}
