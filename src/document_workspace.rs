use std::ops::{Deref, DerefMut};
use std::path::Path;

use crate::document_session::DocumentSession;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DocumentId(u64);

pub struct DocumentTab<'a> {
    pub id: DocumentId,
    pub path: &'a Path,
    pub is_active: bool,
}

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
        self.session_mut(document_id)
    }

    pub fn session(&self, document_id: DocumentId) -> Option<&DocumentSession> {
        let index = self.index_for_id(document_id)?;
        self.documents.get(index).map(|entry| &entry.session)
    }

    pub fn session_mut(&mut self, document_id: DocumentId) -> Option<&mut DocumentSession> {
        let index = self.index_for_id(document_id)?;
        self.documents
            .get_mut(index)
            .map(|entry| &mut entry.session)
    }

    pub fn active_document_id(&self) -> Option<DocumentId> {
        self.active_entry().map(|entry| entry.id)
    }

    pub fn document_tabs(&self) -> Vec<DocumentTab<'_>> {
        self.documents
            .iter()
            .enumerate()
            .map(|(index, entry)| DocumentTab {
                id: entry.id,
                path: entry.session.path.as_path(),
                is_active: self.active_index == Some(index),
            })
            .collect()
    }

    pub fn document_id_for_path(&self, path: &Path) -> Option<DocumentId> {
        let lookup_path = comparable_file_path(path);

        self.documents
            .iter()
            .find(|entry| comparable_file_path(&entry.session.path) == lookup_path)
            .map(|entry| entry.id)
    }

    pub fn open_document(&mut self, session: DocumentSession) -> DocumentId {
        let entry = DocumentEntry {
            id: self.allocate_document_id(),
            session,
        };
        let document_id = entry.id;

        self.documents.push(entry);
        self.active_index = Some(self.documents.len() - 1);

        document_id
    }

    pub fn switch_to(&mut self, document_id: DocumentId) -> bool {
        let Some(index) = self.index_for_id(document_id) else {
            return false;
        };

        self.active_index = Some(index);
        true
    }

    pub fn clear_active_document(&mut self) {
        self.active_index = None;
    }

    pub fn move_document_before(
        &mut self,
        document_id: DocumentId,
        target_document_id: DocumentId,
    ) -> bool {
        self.move_document_to_target(
            document_id,
            target_document_id,
            DocumentMovePosition::Before,
        )
    }

    pub fn move_document_after(
        &mut self,
        document_id: DocumentId,
        target_document_id: DocumentId,
    ) -> bool {
        self.move_document_to_target(document_id, target_document_id, DocumentMovePosition::After)
    }

    fn move_document_to_target(
        &mut self,
        document_id: DocumentId,
        target_document_id: DocumentId,
        position: DocumentMovePosition,
    ) -> bool {
        if document_id == target_document_id {
            return false;
        }

        let Some(source_index) = self.index_for_id(document_id) else {
            return false;
        };
        let Some(target_index) = self.index_for_id(target_document_id) else {
            return false;
        };
        let active_document_id = self.active_document_id();

        let entry = self.documents.remove(source_index);
        let target_index = match position {
            DocumentMovePosition::Before if source_index < target_index => target_index - 1,
            DocumentMovePosition::Before => target_index,
            DocumentMovePosition::After if source_index < target_index => target_index,
            DocumentMovePosition::After => target_index + 1,
        };
        self.documents.insert(target_index, entry);
        self.active_index = active_document_id.and_then(|id| self.index_for_id(id));

        true
    }

    pub fn close(&mut self, document_id: DocumentId) -> Option<DocumentSession> {
        let index = self.index_for_id(document_id)?;
        let entry = self.documents.remove(index);

        self.active_index = match self.active_index {
            Some(active_index) if active_index == index && self.documents.is_empty() => None,
            Some(active_index) if active_index == index => {
                Some(index.min(self.documents.len() - 1))
            }
            Some(active_index) if active_index > index => Some(active_index - 1),
            Some(active_index) => Some(active_index),
            None => None,
        };

        Some(entry.session)
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

    pub fn open_files(&self) -> Vec<&Path> {
        self.documents
            .iter()
            .map(|entry| entry.session.path.as_path())
            .collect()
    }

    pub fn document_ids(&self) -> Vec<DocumentId> {
        self.documents.iter().map(|entry| entry.id).collect()
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

    fn index_for_id(&self, document_id: DocumentId) -> Option<usize> {
        self.documents
            .iter()
            .position(|entry| entry.id == document_id)
    }
}

enum DocumentMovePosition {
    Before,
    After,
}

fn comparable_file_path(path: &Path) -> std::path::PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
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

impl ActiveDocumentSession {
    pub fn id(&self) -> DocumentId {
        self.entry.id
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::document_loader::load_markdown_document;
    use crate::document_session::DocumentSession;

    use super::{DocumentId, DocumentWorkspace};

    static NEXT_TEST_FILE_ID: AtomicUsize = AtomicUsize::new(1);

    #[test]
    fn opens_documents_and_marks_latest_active() {
        let mut workspace = DocumentWorkspace::new();
        let first_path = test_markdown_path("first", "# First");
        let second_path = test_markdown_path("second", "# Second");

        let first_id = workspace.open_document(test_session(&first_path));
        let second_id = workspace.open_document(test_session(&second_path));

        let tabs = workspace.document_tabs();
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0].id, first_id);
        assert_eq!(tabs[0].path, first_path.as_path());
        assert!(!tabs[0].is_active);
        assert_eq!(tabs[1].id, second_id);
        assert_eq!(tabs[1].path, second_path.as_path());
        assert!(tabs[1].is_active);
        assert_eq!(workspace.active_document_id(), Some(second_id));
        assert_eq!(workspace.current_file(), Some(second_path.as_path()));
        assert_eq!(workspace.document_id_for_path(&first_path), Some(first_id));
        assert_eq!(
            workspace.document_id_for_path(&second_path),
            Some(second_id)
        );
        assert_eq!(
            workspace.open_files(),
            vec![first_path.as_path(), second_path.as_path()]
        );
        assert_eq!(workspace.document_ids(), vec![first_id, second_id]);
        assert_eq!(
            workspace
                .session(first_id)
                .map(|session| session.path.as_path()),
            Some(first_path.as_path())
        );
    }

    #[test]
    fn switches_to_existing_document() {
        let mut workspace = DocumentWorkspace::new();
        let first_path = test_markdown_path("first", "# First");
        let second_path = test_markdown_path("second", "# Second");

        let first_id = workspace.open_document(test_session(&first_path));
        workspace.open_document(test_session(&second_path));

        assert!(workspace.switch_to(first_id));
        assert_eq!(workspace.active_document_id(), Some(first_id));
        assert_eq!(workspace.current_file(), Some(first_path.as_path()));
        assert!(!workspace.switch_to(DocumentId(999)));
        assert_eq!(workspace.active_document_id(), Some(first_id));
    }

    #[test]
    fn clears_active_document_without_closing_open_documents() {
        let mut workspace = DocumentWorkspace::new();
        let first_path = test_markdown_path("first", "# First");
        let second_path = test_markdown_path("second", "# Second");

        let first_id = workspace.open_document(test_session(&first_path));
        let second_id = workspace.open_document(test_session(&second_path));

        workspace.clear_active_document();

        assert_eq!(workspace.active_document_id(), None);
        assert_eq!(workspace.current_file(), None);
        assert_eq!(workspace.document_ids(), vec![first_id, second_id]);

        let tabs = workspace.document_tabs();
        assert_eq!(tabs.len(), 2);
        assert!(!tabs[0].is_active);
        assert!(!tabs[1].is_active);
    }

    #[test]
    fn closes_active_document_and_selects_neighbor() {
        let mut workspace = DocumentWorkspace::new();
        let first_path = test_markdown_path("first", "# First");
        let second_path = test_markdown_path("second", "# Second");
        let third_path = test_markdown_path("third", "# Third");

        let first_id = workspace.open_document(test_session(&first_path));
        let second_id = workspace.open_document(test_session(&second_path));
        let third_id = workspace.open_document(test_session(&third_path));

        assert!(workspace.switch_to(second_id));
        let closed = workspace
            .close(second_id)
            .expect("active document should close");

        assert_eq!(closed.path, second_path);
        assert_eq!(workspace.active_document_id(), Some(third_id));

        let tabs = workspace.document_tabs();
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0].id, first_id);
        assert!(!tabs[0].is_active);
        assert_eq!(tabs[1].id, third_id);
        assert!(tabs[1].is_active);
    }

    #[test]
    fn closing_document_before_active_preserves_active_document() {
        let mut workspace = DocumentWorkspace::new();
        let first_path = test_markdown_path("first", "# First");
        let second_path = test_markdown_path("second", "# Second");
        let third_path = test_markdown_path("third", "# Third");

        let first_id = workspace.open_document(test_session(&first_path));
        let second_id = workspace.open_document(test_session(&second_path));
        let third_id = workspace.open_document(test_session(&third_path));

        assert_eq!(workspace.active_document_id(), Some(third_id));
        let closed = workspace.close(second_id).expect("document should close");

        assert_eq!(closed.path, second_path);
        assert_eq!(workspace.active_document_id(), Some(third_id));
        assert_eq!(workspace.current_file(), Some(third_path.as_path()));

        let tabs = workspace.document_tabs();
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0].id, first_id);
        assert_eq!(tabs[1].id, third_id);
        assert!(tabs[1].is_active);
    }

    #[test]
    fn closing_last_active_document_selects_previous_document() {
        let mut workspace = DocumentWorkspace::new();
        let first_path = test_markdown_path("first", "# First");
        let second_path = test_markdown_path("second", "# Second");

        let first_id = workspace.open_document(test_session(&first_path));
        let second_id = workspace.open_document(test_session(&second_path));

        let closed = workspace
            .close(second_id)
            .expect("active document should close");

        assert_eq!(closed.path, second_path);
        assert_eq!(workspace.active_document_id(), Some(first_id));
        assert_eq!(workspace.current_file(), Some(first_path.as_path()));
    }

    #[test]
    fn restores_taken_active_document_to_original_position() {
        let mut workspace = DocumentWorkspace::new();
        let first_path = test_markdown_path("first", "# First");
        let second_path = test_markdown_path("second", "# Second");
        let third_path = test_markdown_path("third", "# Third");

        let first_id = workspace.open_document(test_session(&first_path));
        let second_id = workspace.open_document(test_session(&second_path));
        let third_id = workspace.open_document(test_session(&third_path));

        assert!(workspace.switch_to(second_id));
        let active = workspace
            .take_active_session()
            .expect("active document should be taken");

        assert_eq!(active.id(), second_id);
        assert_eq!(workspace.active_document_id(), Some(third_id));

        workspace.restore_active_session(active);

        let tabs = workspace.document_tabs();
        assert_eq!(tabs.len(), 3);
        assert_eq!(tabs[0].id, first_id);
        assert_eq!(tabs[1].id, second_id);
        assert!(tabs[1].is_active);
        assert_eq!(tabs[2].id, third_id);
    }

    #[test]
    fn moves_document_before_target_and_preserves_active_document() {
        let mut workspace = DocumentWorkspace::new();
        let first_path = test_markdown_path("first", "# First");
        let second_path = test_markdown_path("second", "# Second");
        let third_path = test_markdown_path("third", "# Third");

        let first_id = workspace.open_document(test_session(&first_path));
        let second_id = workspace.open_document(test_session(&second_path));
        let third_id = workspace.open_document(test_session(&third_path));

        assert!(workspace.switch_to(second_id));
        assert!(workspace.move_document_before(third_id, first_id));

        assert_eq!(
            workspace.document_ids(),
            vec![third_id, first_id, second_id]
        );
        assert_eq!(workspace.active_document_id(), Some(second_id));
        assert_eq!(workspace.current_file(), Some(second_path.as_path()));

        let tabs = workspace.document_tabs();
        assert!(!tabs[0].is_active);
        assert!(!tabs[1].is_active);
        assert!(tabs[2].is_active);
    }

    #[test]
    fn moving_document_after_target_preserves_expected_order() {
        let mut workspace = DocumentWorkspace::new();
        let first_path = test_markdown_path("first", "# First");
        let second_path = test_markdown_path("second", "# Second");
        let third_path = test_markdown_path("third", "# Third");

        let first_id = workspace.open_document(test_session(&first_path));
        let second_id = workspace.open_document(test_session(&second_path));
        let third_id = workspace.open_document(test_session(&third_path));

        assert!(workspace.move_document_after(first_id, third_id));

        assert_eq!(
            workspace.document_ids(),
            vec![second_id, third_id, first_id]
        );
        assert_eq!(workspace.active_document_id(), Some(third_id));
    }

    fn test_session(path: &Path) -> DocumentSession {
        let loaded = load_markdown_document(path).expect("test markdown should load");

        DocumentSession::new(
            path.to_path_buf(),
            Arc::clone(&loaded.document),
            loaded.fingerprint,
            loaded.file_snapshot,
        )
    }

    fn test_markdown_path(name: &str, content: &str) -> PathBuf {
        let id = NEXT_TEST_FILE_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("oxidemd_document_workspace_{id}_{name}.md"));
        std::fs::write(&path, content).expect("test markdown should be written");
        path
    }
}
