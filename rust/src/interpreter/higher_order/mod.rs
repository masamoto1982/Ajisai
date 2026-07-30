mod all;
mod any;
mod common;
mod filter;
mod map;

pub(crate) use common::{execute_executable_code, extract_executable_code, ExecutableCode};

pub use all::op_all;
pub use any::op_any;
pub use filter::op_filter;
pub use map::op_map;
