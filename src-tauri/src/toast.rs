use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

/// A counted fact for a pane's banner slot. `id` grows with every toast, so
/// the UI can tell a new message from a snapshot that repeats an old one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Toast {
    pub id: u64,
    pub message: String,
    pub kind: ToastKind,
}
