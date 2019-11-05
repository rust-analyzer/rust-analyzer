//! FIXME: write short doc here

use ra_ide_api::FileId;
use std::collections::HashSet;

#[derive(Default, Debug)]
pub(crate) struct Subscriptions {
    subs: HashSet<FileId>,
}

impl Subscriptions {
    pub(crate) fn add_sub(&mut self, file_id: FileId) {
        self.subs.insert(file_id);
    }
    pub(crate) fn remove_sub(&mut self, file_id: FileId) {
        self.subs.remove(&file_id);
    }
    pub(crate) fn subscriptions(&self) -> Vec<FileId> {
        self.subs.iter().cloned().collect()
    }
}
