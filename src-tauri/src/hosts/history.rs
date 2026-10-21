use super::model::HostEntry;

const MAX_DEPTH: usize = 50;

/// Undo and redo stacks of `entries` snapshots. Undo restores entries only;
/// comment lines and block order stay as they are.
#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Vec<HostEntry>>,
    redo: Vec<Vec<HostEntry>>,
}

impl History {
    /// Records `current` before a change. A snapshot equal to the last one is
    /// skipped, and any redo is dropped.
    pub fn push(&mut self, current: &[HostEntry]) {
        if self.undo.last().is_some_and(|last| last == current) {
            return;
        }
        self.undo.push(current.to_vec());
        if self.undo.len() > MAX_DEPTH {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self, current: &mut Vec<HostEntry>) -> bool {
        let Some(snapshot) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(current, snapshot));
        true
    }

    pub fn redo(&mut self, current: &mut Vec<HostEntry>) -> bool {
        let Some(snapshot) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(current, snapshot));
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    #[cfg(test)]
    pub fn depth(&self) -> usize {
        self.undo.len()
    }

    /// Follows a block rename into every snapshot, so undo after a rename
    /// does not bring back entries in a block no profile names.
    pub fn rename_tag(&mut self, old: &str, new: &str) {
        for entry in self.undo.iter_mut().chain(self.redo.iter_mut()).flatten() {
            if entry.tag.as_deref() == Some(old) {
                entry.tag = Some(new.to_string());
            }
        }
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    pub fn clear_redo(&mut self) {
        self.redo.clear();
    }
}
