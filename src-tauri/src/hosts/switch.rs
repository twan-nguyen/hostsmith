//! Environments exclude each other: switching to one turns every other
//! environment's entries off and writes the file in the same action, so at
//! most one environment is on. Blocks are left as they are.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::state::{Hosts, Outcome, Step};
use crate::profiles::Profile;

/// Read from the entries themselves, so it matches what the resolver sees and
/// survives a relaunch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ActiveState {
    None,
    Active {
        id: Uuid,
    },
    /// More than one environment has entries on: a hand-edited file or an older version.
    Conflict {
        ids: Vec<Uuid>,
    },
}

/// What the caller does when other edits are pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SwitchPolicy {
    /// The window holds the switch for the user to confirm.
    Ask,
    /// The menu bar and the URL scheme write nothing and say why.
    Refuse,
    /// The user accepted writing the pending edits with the switch.
    Include,
}

/// A switch the window is asking about. `target: None` turns environments off.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingSwitch {
    pub target: Option<Uuid>,
    pub pending_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchPreview {
    /// Entries of the target that a switch turns on.
    pub enable: usize,
    /// Entries turned off, per other environment.
    pub disable: Vec<(String, usize)>,
}

impl Hosts {
    fn environments(&self) -> impl Iterator<Item = &Profile> {
        self.profiles.iter().filter(|p| p.is_exclusive)
    }

    fn has_enabled_entries(&self, name: &str) -> bool {
        self.entries
            .iter()
            .any(|e| e.enabled && e.tag.as_deref() == Some(name))
    }

    pub fn active_state(&self) -> ActiveState {
        let on: Vec<Uuid> = self
            .environments()
            .filter(|p| self.has_enabled_entries(&p.name))
            .map(|p| p.id)
            .collect();
        match on.as_slice() {
            [] => ActiveState::None,
            [id] => ActiveState::Active { id: *id },
            _ => ActiveState::Conflict { ids: on },
        }
    }

    fn active_profile(&self) -> Option<&Profile> {
        match self.active_state() {
            ActiveState::Active { id } => self.profiles.iter().find(|p| p.id == id),
            _ => None,
        }
    }

    pub fn switch_preview(&self, target: Option<Uuid>) -> SwitchPreview {
        let name = target
            .and_then(|id| self.environments().find(|p| p.id == id))
            .map(|p| p.name.clone());
        let enable = self
            .entries
            .iter()
            .filter(|e| !e.enabled && e.tag.is_some() && e.tag == name)
            .count();
        let disable = self
            .environments()
            .filter(|p| Some(&p.name) != name.as_ref())
            .map(|p| {
                let count = self
                    .entries
                    .iter()
                    .filter(|e| e.enabled && e.tag.as_deref() == Some(p.name.as_str()))
                    .count();
                (p.name.clone(), count)
            })
            .filter(|(_, count)| *count > 0)
            .collect();
        SwitchPreview { enable, disable }
    }

    /// Switches to one environment, or with `None` turns every environment
    /// off. When other edits are pending, `policy` decides whether they are
    /// written too.
    pub fn switch_environment(&mut self, target: Option<Uuid>, policy: SwitchPolicy) -> Step {
        if let Some(reason) = self.write_blocker() {
            return self.blocked(reason);
        }
        let target_profile =
            target.and_then(|id| self.environments().find(|p| p.id == id).cloned());
        if target.is_some()
            && target_profile
                .as_ref()
                .is_none_or(|p| self.tag_entry_count(&p.name) == 0)
        {
            return Step::Done(Outcome::NothingToDo);
        }

        // A caller that cannot ask leaves the editor's typing where it is.
        if policy == SwitchPolicy::Refuse && self.dirty() {
            return Step::Done(Outcome::Refused(self.pending_count()));
        }
        self.merge_raw_draft();
        if self.dirty() {
            let count = self.pending_count();
            match policy {
                SwitchPolicy::Ask => {
                    self.pending_switch = Some(PendingSwitch {
                        target,
                        pending_count: count,
                    });
                    return Step::Done(Outcome::NeedsConfirmation(count));
                }
                SwitchPolicy::Refuse => return Step::Done(Outcome::Refused(count)),
                SwitchPolicy::Include => {}
            }
        }

        let target_name = target_profile.as_ref().map(|p| p.name.clone());
        let others: Vec<String> = self
            .environments()
            .map(|p| p.name.clone())
            .filter(|name| Some(name) != target_name.as_ref())
            .collect();
        let mut updated = self.entries.clone();
        for entry in &mut updated {
            let Some(tag) = &entry.tag else { continue };
            if Some(tag) == target_name.as_ref() {
                entry.enabled = true;
            } else if others.contains(tag) {
                entry.enabled = false;
            }
        }
        if updated == self.entries && !self.dirty() {
            return Step::Done(Outcome::NothingToDo);
        }

        let toast = match (&target_profile, self.active_profile()) {
            (Some(target), _) => format!("Đã chuyển sang {}", target.name),
            (None, Some(previous)) => format!("Đã tắt {}", previous.name),
            (None, None) => "Đã tắt các môi trường".to_string(),
        };
        let before = std::mem::replace(&mut self.entries, updated);
        Step::Write(self.begin_switch_write(before, toast))
    }

    /// Runs the switch the window asked about, writing the pending edits with it.
    pub fn confirm_pending_switch(&mut self) -> Step {
        match self.pending_switch.take() {
            Some(pending) => self.switch_environment(pending.target, SwitchPolicy::Include),
            None => Step::Done(Outcome::NothingToDo),
        }
    }

    pub fn cancel_pending_switch(&mut self) {
        self.pending_switch = None;
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::tests::hosts;
    use super::super::state::{EntryInput, WriteJob};
    use super::*;
    use crate::platform::RunOutcome;
    use crate::profiles::ProfileColor;

    const FIXTURE: &str = "127.0.0.1\tlocalhost

## [tag:Local]
127.0.0.1\tapi.test
## [/tag:Local]

## [tag:Staging]
# 10.0.0.5\tapi.test
## [/tag:Staging]

## [tag:Docker]
127.0.0.1\tdb.test
## [/tag:Docker]
";

    fn profiles() -> Vec<Profile> {
        vec![
            Profile::new("Local", ProfileColor::Blue, true),
            Profile::new("Staging", ProfileColor::Amber, true),
            Profile::new("Docker", ProfileColor::Purple, false),
        ]
    }

    fn make(content: &str) -> Hosts {
        hosts(content, profiles()).0
    }

    fn id(h: &Hosts, name: &str) -> Uuid {
        h.profiles.iter().find(|p| p.name == name).unwrap().id
    }

    fn enabled(h: &Hosts, tag: &str) -> Vec<bool> {
        h.entries
            .iter()
            .filter(|e| e.tag.as_deref() == Some(tag))
            .map(|e| e.enabled)
            .collect()
    }

    /// Records every write a switch makes and answers with `answer`.
    struct Writes {
        contents: Vec<String>,
        answer: RunOutcome,
    }

    impl Writes {
        fn ok() -> Self {
            Self {
                contents: Vec::new(),
                answer: RunOutcome::Ok,
            }
        }

        fn run(&mut self, h: &mut Hosts, step: Step) -> Outcome {
            match step {
                Step::Done(outcome) => outcome,
                Step::Write(job) => self.finish(h, job),
            }
        }

        fn finish(&mut self, h: &mut Hosts, job: WriteJob) -> Outcome {
            self.contents.push(job.content.clone());
            h.finish_write(job, self.answer.clone())
        }
    }

    fn extra() -> EntryInput {
        EntryInput {
            ip: "127.0.0.1".into(),
            hostname: "extra.test".into(),
            ..Default::default()
        }
    }

    #[test]
    fn the_active_environment_is_read_from_the_entries() {
        let h = make(FIXTURE);
        assert_eq!(
            h.active_state(),
            ActiveState::Active {
                id: id(&h, "Local")
            }
        );
    }

    #[test]
    fn switching_turns_the_other_environment_off_and_writes_once() {
        let mut h = make(FIXTURE);
        let mut writes = Writes::ok();
        let step = h.switch_environment(Some(id(&h, "Staging")), SwitchPolicy::Ask);
        assert_eq!(writes.run(&mut h, step), Outcome::Written);
        assert_eq!(writes.contents.len(), 1);
        assert_eq!(enabled(&h, "Local"), [false]);
        assert_eq!(enabled(&h, "Staging"), [true]);
        assert_eq!(enabled(&h, "Docker"), [true], "blocks are left alone");
        assert_eq!(
            h.active_state(),
            ActiveState::Active {
                id: id(&h, "Staging")
            }
        );
        assert!(!h.dirty());
        assert!(writes.contents[0].contains("10.0.0.5\tapi.test"));
        assert!(writes.contents[0].contains("# 127.0.0.1\tapi.test"));
        assert_eq!(h.toast().unwrap().message, "Đã chuyển sang Staging");
    }

    #[test]
    fn deactivating_turns_the_environment_off_and_writes() {
        let mut h = make(FIXTURE);
        let mut writes = Writes::ok();
        let step = h.switch_environment(None, SwitchPolicy::Ask);
        assert_eq!(writes.run(&mut h, step), Outcome::Written);
        assert_eq!(h.active_state(), ActiveState::None);
        assert_eq!(enabled(&h, "Docker"), [true]);
        assert_eq!(h.toast().unwrap().message, "Đã tắt Local");
    }

    #[test]
    fn two_environments_on_is_a_conflict_a_switch_resolves() {
        let mut h = make(&FIXTURE.replace("# 10.0.0.5", "10.0.0.5"));
        assert_eq!(
            h.active_state(),
            ActiveState::Conflict {
                ids: vec![id(&h, "Local"), id(&h, "Staging")]
            }
        );
        let step = h.switch_environment(Some(id(&h, "Local")), SwitchPolicy::Ask);
        Writes::ok().run(&mut h, step);
        assert_eq!(
            h.active_state(),
            ActiveState::Active {
                id: id(&h, "Local")
            }
        );
    }

    #[test]
    fn blocks_and_empty_environments_cannot_be_switched_to() {
        let mut h = make(FIXTURE);
        let mut writes = Writes::ok();
        let step = h.switch_environment(Some(id(&h, "Docker")), SwitchPolicy::Ask);
        assert_eq!(writes.run(&mut h, step), Outcome::NothingToDo);
        let qa = h.add_profile("QA", ProfileColor::Amber, true).unwrap();
        let step = h.switch_environment(Some(qa.id), SwitchPolicy::Ask);
        assert_eq!(writes.run(&mut h, step), Outcome::NothingToDo);
        assert!(writes.contents.is_empty());
    }

    #[test]
    fn switching_to_the_active_environment_writes_nothing() {
        let mut h = make(FIXTURE);
        let step = h.switch_environment(Some(id(&h, "Local")), SwitchPolicy::Ask);
        assert!(matches!(step, Step::Done(Outcome::NothingToDo)));
    }

    #[test]
    fn pending_edits_are_asked_about_or_refused() {
        let mut h = make(FIXTURE);
        let mut writes = Writes::ok();
        h.add_entry(extra()).unwrap();
        let staging = id(&h, "Staging");

        let step = h.switch_environment(Some(staging), SwitchPolicy::Refuse);
        assert_eq!(writes.run(&mut h, step), Outcome::Refused(1));
        assert!(h.pending_switch.is_none());

        let step = h.switch_environment(Some(staging), SwitchPolicy::Ask);
        assert_eq!(writes.run(&mut h, step), Outcome::NeedsConfirmation(1));
        assert_eq!(h.pending_switch.as_ref().unwrap().target, Some(staging));
        assert!(writes.contents.is_empty());

        let step = h.confirm_pending_switch();
        assert_eq!(writes.run(&mut h, step), Outcome::Written);
        assert!(h.pending_switch.is_none());
        assert_eq!(writes.contents.len(), 1);
        assert!(writes.contents[0].contains("extra.test"));
        assert_eq!(h.active_state(), ActiveState::Active { id: staging });
    }

    #[test]
    fn cancelling_the_question_writes_nothing() {
        let mut h = make(FIXTURE);
        h.add_entry(extra()).unwrap();
        let step = h.switch_environment(Some(id(&h, "Staging")), SwitchPolicy::Ask);
        assert!(matches!(step, Step::Done(Outcome::NeedsConfirmation(_))));
        h.cancel_pending_switch();
        assert!(h.pending_switch.is_none());
        assert_eq!(
            h.active_state(),
            ActiveState::Active {
                id: id(&h, "Local")
            }
        );
    }

    #[test]
    fn a_cancelled_prompt_puts_the_entries_back() {
        let mut h = make(FIXTURE);
        let before = h.entries.clone();
        let mut writes = Writes {
            contents: Vec::new(),
            answer: RunOutcome::Cancelled,
        };
        let step = h.switch_environment(Some(id(&h, "Staging")), SwitchPolicy::Ask);
        assert_eq!(writes.run(&mut h, step), Outcome::Cancelled);
        assert_eq!(h.entries, before);
        assert!(!h.dirty());
        assert!(!h.history.can_undo());
    }

    #[test]
    fn a_failed_write_reports_and_puts_the_entries_back() {
        let mut h = make(FIXTURE);
        let message = "quyền quản trị bị từ chối (mã 1)".to_string();
        let mut writes = Writes {
            contents: Vec::new(),
            answer: RunOutcome::Denied(1),
        };
        let step = h.switch_environment(Some(id(&h, "Staging")), SwitchPolicy::Ask);
        assert_eq!(writes.run(&mut h, step), Outcome::Failed(message));
        assert_eq!(
            h.active_state(),
            ActiveState::Active {
                id: id(&h, "Local")
            }
        );
    }

    #[test]
    fn typing_in_the_raw_editor_is_folded_in_not_lost() {
        let mut h = make(FIXTURE);
        let mut writes = Writes::ok();
        h.set_raw_draft(
            Some(format!("{}127.0.0.1\ttyped.test\n", h.serialize())),
            &h.serialize(),
        );
        let step = h.switch_environment(Some(id(&h, "Staging")), SwitchPolicy::Ask);
        assert!(matches!(step, Step::Done(Outcome::NeedsConfirmation(_))));
        assert!(h.entries.iter().any(|e| e.hostname == "typed.test"));
        let step = h.confirm_pending_switch();
        writes.run(&mut h, step);
        assert!(writes.contents[0].contains("typed.test"));
    }

    #[test]
    fn editor_text_that_matches_the_model_is_not_pending() {
        let mut h = make(FIXTURE);
        let mut writes = Writes::ok();
        h.set_raw_draft(Some(h.serialize()), &h.serialize());
        let step = h.switch_environment(Some(id(&h, "Staging")), SwitchPolicy::Refuse);
        assert_eq!(writes.run(&mut h, step), Outcome::Written);
    }

    #[test]
    fn preview_counts_what_a_switch_turns_on_and_off() {
        let h = make(FIXTURE);
        let preview = h.switch_preview(Some(id(&h, "Staging")));
        assert_eq!(preview.enable, 1);
        assert_eq!(preview.disable, [("Local".to_string(), 1)]);
        assert_eq!(h.switch_preview(None).enable, 0);
    }

    #[test]
    fn blocks_found_in_the_file_start_independent() {
        let h = make(&format!(
            "{FIXTURE}\n## [tag:Mock]\n127.0.0.1\tmock.test\n## [/tag:Mock]\n"
        ));
        assert!(
            !h.profiles
                .iter()
                .find(|p| p.name == "Mock")
                .unwrap()
                .is_exclusive
        );
    }

    #[test]
    fn one_switch_at_a_time() {
        let mut h = make(FIXTURE);
        let Step::Write(job) = h.switch_environment(Some(id(&h, "Staging")), SwitchPolicy::Ask)
        else {
            panic!("expected a write")
        };
        let second = h.switch_environment(None, SwitchPolicy::Ask);
        assert!(matches!(second, Step::Done(Outcome::Failed(_))));
        assert!(h.add_entry(extra()).is_err(), "edits wait for the write");
        Writes::ok().finish(&mut h, job);
    }

    #[test]
    fn a_switch_that_landed_can_be_undone() {
        let mut h = make(FIXTURE);
        let step = h.switch_environment(Some(id(&h, "Staging")), SwitchPolicy::Ask);
        Writes::ok().run(&mut h, step);
        h.undo().unwrap();
        assert_eq!(
            h.active_state(),
            ActiveState::Active {
                id: id(&h, "Local")
            }
        );
        assert!(h.dirty(), "the undo is pending until applied");
    }

    #[test]
    fn a_refusing_caller_leaves_the_editor_typing_alone() {
        let mut h = make(FIXTURE);
        let ids: Vec<Uuid> = h.entries.iter().map(|e| e.id).collect();
        h.set_raw_draft(
            Some(format!("{}127.0.0.1\ttyped.test\n", h.serialize())),
            &h.serialize(),
        );
        let step = h.switch_environment(Some(id(&h, "Staging")), SwitchPolicy::Refuse);
        assert!(matches!(step, Step::Done(Outcome::Refused(1))));
        assert_eq!(h.entries.iter().map(|e| e.id).collect::<Vec<_>>(), ids);
        assert!(!h.history.can_undo());
        assert!(h.dirty(), "the typing is still pending");
    }

    #[test]
    fn a_rolled_back_switch_with_a_full_undo_stack_keeps_the_history() {
        let mut h = make(FIXTURE);
        let docker = h
            .entries
            .iter()
            .find(|e| e.hostname == "db.test")
            .unwrap()
            .id;
        for _ in 0..50 {
            h.toggle_entry(docker).unwrap();
        }
        let Step::Write(job) = h.switch_environment(Some(id(&h, "Staging")), SwitchPolicy::Include)
        else {
            panic!("expected a write")
        };
        h.finish_write(job, RunOutcome::Cancelled);
        assert_eq!(h.history.depth(), 50);
        h.undo().unwrap();
        assert_eq!(
            enabled(&h, "Docker"),
            [false],
            "the last toggle is undone, not the switch"
        );
    }
}
