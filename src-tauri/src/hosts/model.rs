use serde::Serialize;
use uuid::Uuid;

/// One `ip hostname [alias…]` line of the hosts file, on or commented out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostEntry {
    pub id: Uuid,
    pub ip: String,
    pub hostname: String,
    /// Further hostnames on the same line, kept so a write never drops them.
    pub aliases: Vec<String>,
    pub comment: String,
    pub enabled: bool,
    /// The `## [tag:Name]` block the entry sits in; `None` is outside every block.
    pub tag: Option<String>,
}

impl HostEntry {
    pub fn new(ip: &str, hostname: &str, comment: &str, tag: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            ip: ip.to_string(),
            hostname: hostname.to_string(),
            aliases: Vec::new(),
            comment: comment.to_string(),
            enabled: true,
            tag,
        }
    }

    pub fn hostnames(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.hostname.as_str()).chain(self.aliases.iter().map(String::as_str))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

/// A counted fact for the pane's banner slot. `id` grows with every toast, so
/// the UI can tell a new message from a snapshot that repeats an old one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Toast {
    pub id: u64,
    pub message: String,
    pub kind: ToastKind,
}
