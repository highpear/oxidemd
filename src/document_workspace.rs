use std::path::Path;

use crate::document_session::DocumentSession;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentId(u64);

pub struct DocumentWorkspace {
    next_document_id: u64,
    active_document_id: Option<DocumentId>,
    active_session: Option<DocumentSession>,
}

impl DocumentWorkspace {
    pub fn new() -> Self {
        Self {
            next_document_id: 1,
            active_document_id: None,
            active_session: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.active_session.is_none()
    }

    pub fn active_session(&self) -> Option<&DocumentSession> {
        self.active_session.as_ref()
    }

    pub fn active_session_mut(&mut self) -> Option<&mut DocumentSession> {
        self.active_session.as_mut()
    }

    pub fn active_session_mut_for_id(
        &mut self,
        document_id: DocumentId,
    ) -> Option<&mut DocumentSession> {
        if self.active_document_id == Some(document_id) {
            return self.active_session.as_mut();
        }

        None
    }

    pub fn active_document_id(&self) -> Option<DocumentId> {
        self.active_document_id
    }

    pub fn set_active_session(&mut self, session: DocumentSession) {
        self.active_document_id = Some(self.allocate_document_id());
        self.active_session = Some(session);
    }

    pub fn clear_active_session(&mut self) {
        self.active_document_id = None;
        self.active_session = None;
    }

    pub fn take_active_session(&mut self) -> Option<DocumentSession> {
        self.active_session.take()
    }

    pub fn restore_active_session(&mut self, session: DocumentSession) {
        self.active_session = Some(session);
    }

    pub fn current_file(&self) -> Option<&Path> {
        self.active_session
            .as_ref()
            .map(|session| session.path.as_path())
    }

    fn allocate_document_id(&mut self) -> DocumentId {
        let document_id = DocumentId(self.next_document_id);
        self.next_document_id += 1;
        document_id
    }
}
