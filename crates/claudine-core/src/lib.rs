//! Cœur logique de Claudine : lecture/écriture de la structure `~/.claude`.

pub mod error;
pub mod home;
pub mod pathcodec;

pub use error::{CoreError, Report, Result};
pub use home::ClaudeHome;
pub use pathcodec::encode_cwd;
