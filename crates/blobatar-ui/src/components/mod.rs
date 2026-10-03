mod agents;
mod chat;
mod password;
mod presence;
mod profile;
mod users;

pub use agents::{Agent, AgentList, AgentSelected};
pub use chat::{ChatMessage, GroupChat, message_runs};
pub use password::{PasswordEdited, PasswordField};
pub use presence::{PresenceAvatar, PresenceState};
pub use profile::ProfileAvatar;
pub use users::{User, UserTable};

use blobatar_core::{Avatar, Options};
use blobatar_gpui::Drawing;
use std::sync::Arc;

fn drawing(name: &str, options: &Options) -> Arc<Drawing> {
    Arc::new(Drawing::new(&Avatar::new(name, options), options))
}
