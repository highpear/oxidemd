use std::path::Path;

use crate::document_session::DocumentSession;

pub struct DocumentWorkspace {
    active_session: Option<DocumentSession>,
}

impl DocumentWorkspace {
    pub fn new() -> Self {
        Self {
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

    pub fn set_active_session(&mut self, session: DocumentSession) {
        self.active_session = Some(session);
    }

    pub fn clear_active_session(&mut self) {
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
}
