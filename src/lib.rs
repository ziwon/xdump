use lazy_static::lazy_static;
use slog::{self, o, Drain, Logger};
use slog_async;
use slog_term;

#[macro_export]
macro_rules! func_name {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let full_name = type_name_of(f).trim_end_matches("{{closure}}::f");
        let parts: Vec<&str> = full_name.split("::").collect();
        if parts.len() > 2 {
            parts[parts.len() - 2]
        } else {
            full_name // FIXME: 풀넴말고 다른 거?
        }
    }};
}

#[macro_export]
macro_rules! xlog {
    ($level:expr, $msg:expr) => {{
        let logger = &$crate::LOGGER;
        let func_name = $crate::func_name!();
        let struct_name = module_path!().split("::").last().unwrap();

        match $level {
            'I' => {
                slog::info!(logger, "{}::{}() - {}", struct_name, func_name, $msg);
            }
            'D' => {
                slog::debug!(logger, "{}::{}() - {}", struct_name, func_name, $msg);
            }
            'E' => {
                slog::error!(logger, "{}::{}() - {}", struct_name, func_name, $msg);
            }
            'W' => {
                slog::warn!(logger, "{}::{}() - {}", struct_name, func_name, $msg);
            }
            _ => {} // do nothing
        }
    }};
}

lazy_static! {
    pub static ref LOGGER: Logger = {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::CompactFormat::new(decorator)
            .use_custom_timestamp(move |out| {
                write!(
                    out,
                    "{}",
                    chrono::Local::now()
                        .with_timezone(&chrono::FixedOffset::east_opt(9 * 3600).unwrap())
                        .format("%Y-%m-%d %H:%M:%S")
                )
            })
            .build()
            .fuse();
        let drain = slog_async::Async::new(drain).chan_size(1000).build().fuse();

        Logger::root(drain, o!())
    };
}
