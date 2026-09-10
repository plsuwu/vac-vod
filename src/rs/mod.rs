pub mod argparser;
pub mod eval;
pub mod fetch;
pub mod ingest;
pub mod util;

pub mod prelude {
    pub use crate::rs::argparser::{ActionArg, get_args};
    pub use crate::rs::fetch::fetch_uploads;
    pub use crate::rs::util::tracing::init_stdout_logger;
    pub use crate::rs::util::paths::ChannelDirectory;
    pub use crate::rs::util::read_file;

    pub type Word = (f64, String);
}
