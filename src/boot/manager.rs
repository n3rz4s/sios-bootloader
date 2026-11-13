use super::{
    config::BootConfig,
    entry::{BootEntry, BootEntryId},
    error::{BootError, BootResult},
};

#[derive(Debug)]
pub struct BootManager {
    config: BootConfig,

    selected_index: usize,
}

impl BootManager {
    pub fn new(config: BootConfig) -> BootResult<Self> {
        let entries = config.entries();

        if entries.is_empty() {
            return Err(BootError::NoEntries);
        }

        let selected_index = if let Some(default_id) = config.default_entry() {
            entries.iter().position(|entry| entry.id() == default_id)
        } else {
            entries.iter().position(|entry| entry.is_enabled())
        };

        let selected_index = selected_index.ok_or(BootError::NoBootableEntries)?;

        Ok(Self {
            config,
            selected_index,
        })
    }

    pub fn entries(&self) -> &[BootEntry] {
        self.config.entries()
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn selected_entry(&self) -> &BootEntry {
        &self.config.entries()[self.selected_index]
    }

    pub fn timeout_seconds(&self) -> u16 {
        self.config.timeout_seconds()
    }

    pub fn select_next(&mut self) -> &BootEntry {
        self.move_selection(true);

        self.selected_entry()
    }

    pub fn select_previous(&mut self) -> &BootEntry {
        self.move_selection(false);

        self.selected_entry()
    }

    pub fn select_by_id(&mut self, id: BootEntryId) -> BootResult<&BootEntry> {
        let index = self
            .config
            .entries()
            .iter()
            .position(|entry| entry.id() == id)
            .ok_or(BootError::EntryNotFound)?;

        if !self.config.entries()[index].is_enabled() {
            return Err(BootError::EntryDisabled);
        }

        self.selected_index = index;

        Ok(self.selected_entry())
    }

    fn move_selection(&mut self, forward: bool) {
        let count = self.config.entries().len();

        for step in 1..=count {
            let candidate = if forward {
                (self.selected_index + step) % count
            } else {
                (self.selected_index + count - (step % count)) % count
            };

            if self.config.entries()[candidate].is_enabled() {
                self.selected_index = candidate;

                return;
            }
        }
    }
}
