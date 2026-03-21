pub struct SnapshotStore;

impl SnapshotStore {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SnapshotStore {
    fn default() -> Self {
        Self::new()
    }
}
