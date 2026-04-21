#![feature(string_remove_matches)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::panic::PanicHookInfo;
use tracing::level_filters::LevelFilter;

use crate::ui::App;

mod ui;
mod message;
mod messangers;
mod message_server;
mod send_categories;
mod appdata;

fn panic_message_box(info: &PanicHookInfo) {
    rfd::MessageDialog::new()
    .set_buttons(rfd::MessageButtons::Ok)
    .set_title("Виявлена критична помилка")
    .set_level(rfd::MessageLevel::Error)
    .set_description(info.to_string())
    .show();
}

fn main() {
    std::panic::set_hook(Box::new(panic_message_box));
    let have_home = create_save_folder().is_ok();

    #[cfg(debug_assertions)]
    {
        tracing_subscriber::FmtSubscriber::builder()
        .pretty()
        .with_max_level(LevelFilter::INFO)
        .with_writer(std::io::stdout)
        .init();
    }

    #[cfg(not(debug_assertions))]
    {
        use std::path::PathBuf;
        let path = if have_home {
            let mut home = std::env::home_dir().unwrap();
            home.extend([".sender", "sender.log"]);
            home
        }
        else {
            PathBuf::from("sender.log")
        };

        let log_file = std::fs::File::create(&path).unwrap();
        tracing_subscriber::FmtSubscriber::builder()
        .pretty()
        .with_max_level(LevelFilter::WARN)
        .with_writer(log_file)
        .with_ansi(false)
        .init();
    }

    let title = concat!(
        "Starting Modern Sender v",
        env!("CARGO_PKG_VERSION"),
    );
    tracing::warn!("Starting {}", title);

    iced::application::timed(
        App::new,
        App::update,
        App::subscription,
        App::view,
    )
    .title(title)
    .theme(App::theme)
    .font(include_bytes!("Roboto-VariableFont_wdth,wght.ttf"))
    .font(include_bytes!("ui/icons/MaterialIcons-Regular.ttf"))
    .default_font(iced::Font::with_name("Roboto"))
    .exit_on_close_request(false)
    .run()
    .unwrap()
}

fn create_save_folder() -> anyhow::Result<()> {
    let mut home = std::env::home_dir().ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?;
    home.push(".sender");
    std::fs::create_dir(&home)?;
    Ok(())
}
