use std::ops::{Deref, DerefMut};
use std::path::Path;

use crate::document_session::DocumentSession;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentId(u64);

pub struct DocumentWorkspace {
    next_document_id: u64,
    active_index: Option<usize>,
    documents: Vec<DocumentEntry>,
}

pub struct ActiveDocumentSession {
    index: usize,
    entry: DocumentEntry,
}

struct DocumentEntry {
    id: DocumentId,
    session: DocumentSession,
}

impl DocumentWorkspace {
    pub fn new() -> Self {
        Self {
            next_document_id: 1,
            active_index: None,
            documents: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    pub fn active_session(&self) -> Option<&DocumentSession> {
        self.active_entry().map(|entry| &entry.session)
    }

    pub fn active_session_mut(&mut self) -> Option<&mut DocumentSession> {
        self.active_entry_mut().map(|entry| &mut entry.session)
    }

    pub fn active_session_mut_for_id(
        &mut self,
        document_id: DocumentId,
    ) -> Option<&mut DocumentSession> {
        self.documents
            .iter_mut()
            .find(|entry| entry.id == document_id)
            .map(|entry| &mut entry.session)
    }

    pub fn active_document_id(&self) -> Option<DocumentId> {
        self.active_entry().map(|entry| entry.id)
    }

    pub fn set_active_session(&mut self, session: DocumentSession) {
        let entry = DocumentEntry {
            id: self.allocate_document_id(),
            session,
        };

        self.documents.clear();
        self.documents.push(entry);
        self.active_index = Some(0);
    }

    pub fn clear_active_session(&mut self) {
        self.documents.clear();
        self.active_index = None;
    }

    pub fn take_active_session(&mut self) -> Option<ActiveDocumentSession> {
        let index = self.active_index()?;
        let entry = self.documents.remove(index);
        self.active_index = if self.documents.is_empty() {
            None
        } else {
            Some(index.min(self.documents.len() - 1))
        };

        Some(ActiveDocumentSession { index, entry })
    }

    pub fn restore_active_session(&mut self, active_session: ActiveDocumentSession) {
        if let Some(index) = self
            .documents
            .iter()
            .position(|entry| entry.id == active_session.entry.id)
        {
            self.documents[index] = active_session.entry;
            self.active_index = Some(index);
            return;
        }

        let index = active_session.index.min(self.documents.len());
        self.documents.insert(index, active_session.entry);
        self.active_index = Some(index);
    }

    pub fn current_file(&self) -> Option<&Path> {
        self.active_session().map(|session| session.path.as_path())
    }

    fn allocate_document_id(&mut self) -> DocumentId {
        let document_id = DocumentId(self.next_document_id);
        self.next_document_id += 1;
        document_id
    }

    fn active_entry(&self) -> Option<&DocumentEntry> {
        self.active_index()
            .and_then(|index| self.documents.get(index))
    }

    fn active_entry_mut(&mut self) -> Option<&mut DocumentEntry> {
        let index = self.active_index()?;
        self.documents.get_mut(index)
    }

    fn active_index(&self) -> Option<usize> {
        self.active_index
            .filter(|index| *index < self.documents.len())
    }
}

impl Deref for ActiveDocumentSession {
    type Target = DocumentSession;

    fn deref(&self) -> &Self::Target {
        &self.entry.session
    }
}

impl DerefMut for ActiveDocumentSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entry.session
    }
}
