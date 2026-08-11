pub mod argparser;
pub mod fetch;
pub mod util;
pub mod tokenize;

pub mod prelude {
    pub use crate::rs::argparser::{ActionArg, get_args};
    pub use crate::rs::fetch::fetch_uploads;
}
