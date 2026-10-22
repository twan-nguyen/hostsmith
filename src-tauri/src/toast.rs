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

/// The latest toast of one model, numbered in the order they were shown.
#[derive(Debug, Default)]
pub struct Toasts {
    current: Option<Toast>,
    seq: u64,
}

impl Toasts {
    pub fn show(&mut self, message: String, kind: ToastKind) {
        self.seq += 1;
        self.current = Some(Toast {
            id: self.seq,
            message,
            kind,
        });
    }

    pub fn current(&self) -> Option<&Toast> {
        self.current.as_ref()
    }

    #[cfg(test)]
    pub fn clear(&mut self) {
        self.current = None;
    }
}
