mod prompts;
mod status;
mod zones;

pub use prompts::StickyPromptInfo;
pub use status::ShellIntegrationStatus;
pub(crate) use status::{ShellIntegrationAlertHandler, ShellIntegrationState};
