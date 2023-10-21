#[derive(Copy, Clone)]
pub(crate) enum ReaderState {
    Size,
    Data
}

pub(crate) struct ReaderController {
    state: ReaderState,
    next_size: usize,
}

impl ReaderController {
    pub(crate) fn new() -> Self {
        Self {
            state: ReaderState::Size,
            next_size: 8,
        }
    }

    pub(crate) fn received_size(&mut self, size: usize) {
        self.next_size = size;
        self.state = ReaderState::Data;
    }

    pub(crate) fn received_data(&mut self) {
        self.next_size = 8;
        self.state = ReaderState::Size;
    }

    pub(crate) fn get_next_size(&self) -> usize {
        self.next_size
    }

    pub(crate) fn get_state(&self) -> ReaderState {
        self.state
    }
}