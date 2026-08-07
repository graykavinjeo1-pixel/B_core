#[derive(Debug, Clone, Default)]
pub struct ColdMemoryArchive {
    archived_count: usize,
}

impl ColdMemoryArchive {
    pub fn archived_count(&self) -> usize {
        self.archived_count
    }

    pub fn archive(&mut self, count: usize) {
        self.archived_count += count;
    }
}
