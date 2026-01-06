macro_rules! icon {
    ($name:ident, $file:literal) => {
        pub const $name: &[u8] = include_bytes!($file);
    };
}

icon!(DELETE, "icons/delete.svg");
icon!(EDIT, "icons/edit.svg");
icon!(SETTINGS, "icons/settings.svg");
icon!(SIGNAL, "icons/signal.svg");
icon!(WHASTAPP, "icons/whatsapp.svg");
icon!(DROP_DOWN, "icons/drop_down.svg");
icon!(DROP_UP, "icons/drop_up.svg");
icon!(ARROW_BACK, "icons/arrow_back.svg");

