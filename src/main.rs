#![feature(string_remove_matches)]

use std::{fs::File, sync::Arc};
use tracing::level_filters::LevelFilter;

use crate::ui::App;

mod ui;
mod message;
mod messangers;
mod message_server;
mod send_categories;
#[cfg(test)]
mod test;


fn main() {
    let log_file = File::create("sender.log").unwrap();
    #[cfg(debug_assertions)]
    let log_filter = LevelFilter::INFO;
    #[cfg(not(debug_assertions))]
    let log_filter = LevelFilter::WARN;


    tracing_subscriber::FmtSubscriber::builder()
    .pretty()
    .with_max_level(log_filter)
    .with_writer(log_file)
    .with_writer(Arc::new(std::io::stdout()))
    .init();

    iced::application::timed(
        App::new,
        App::update,
        App::subscription,
        App::view,
    )
    .title("Modern Sender")
    .theme(App::theme)
    .font(include_bytes!("Roboto-VariableFont_wdth,wght.ttf"))
    .font(include_bytes!("MaterialIcons-Regular.ttf"))
    .default_font(iced::Font::with_name("Roboto"))
    .exit_on_close_request(false)
    .run()
    .unwrap()
}
