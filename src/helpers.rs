use slint::{ToSharedString, Weak};
use crate::App;


pub trait ResultExtUnit {
    fn report_error(self, app_handle: Weak<App>);
}

impl<E: ToSharedString + Send + 'static> ResultExtUnit for Result<(), E> {
    fn report_error(self, app_handle: Weak<App>) {
        if let Err(e) = self {
            app_handle.upgrade_in_event_loop(move |app| app.invoke_report(e.to_shared_string())).expect(".report_error() called after app closed")
        }
    }
}

pub trait ResultExtInspect {
    fn inspect_err_with_report(self, app_handle: Weak<App>) -> Self;
}

impl<T, E: ToSharedString + Send + 'static> ResultExtInspect for Result<T, E> {
    fn inspect_err_with_report(self, app_handle: Weak<App>) -> Self {
        if let Err(ref e) = self {
            let err_str = e.to_shared_string();
            log::error!("Error: {}", &err_str);
            app_handle.upgrade_in_event_loop(move |app| app.invoke_report(err_str)).expect(".inspect_with_report() called after app closed");
        }

        self
    }
}

#[macro_export]
macro_rules! report {
    ($app_handle:expr, $s:expr $(,)?) => {
        $app_handle.upgrade_in_event_loop(move |app| app.invoke_report($s.to_shared_string())).expect(".report!() called after app closed");
    };
    ($app_handle:expr, $s:literal $(, $vars:expr)* $(,)?) => {
        $app_handle.upgrade_in_event_loop(move |app| app.invoke_report(slint::format!($s $(, $vars)*))).expect(".report!() called after app closed");
    };
}

macro_rules! report_log_level {
    (info, $t:expr) => {
        log::info!("{}", $t);
    };
    (warn, $t:expr) => {
        log::warn!("{}", $t);
    };
    (error, $t:expr) => {
        log::error!("{}", $t);
    };
    ($wrong:ident, $t:expr) => {
        compile_error!("Level accepts only following: info, warn, error");
    };
}

macro_rules! report_log_format_str {
    (info, $t:expr) => {
        slint::format!("Info: {}", $t)
    };
    (warn, $t:expr) => {
        slint::format!("Warn: {}", $t)
    };
    (error, $t:expr) => {
        slint::format!("Error: {}", $t)
    };
    ($wrong:ident, $t:expr) => {
        compile_error!("Level accepts only following: info, warn, error");
    };
}

#[macro_export]
macro_rules! report_log {
    ($level:ident, $app_handle:expr, $s:expr $(,)?) => {
        {
            report_log_level!($level, &$s);
            $app_handle.upgrade_in_event_loop(move |app| app.invoke_report(report_log_format_str!($level, s))).expect(".report_log!() called after app closed");
        }
    };
    ($level:ident, $app_handle:expr, $s:literal $(, $vars:expr)* $(,)?) => {
        {
            let s = slint::format!($s $(, $vars)*);
            report_log_level!($level, &s);
            $app_handle.upgrade_in_event_loop(move |app| app.invoke_report(report_log_format_str!($level, s))).expect(".report_log!() called after app closed");
        }
    };
}

