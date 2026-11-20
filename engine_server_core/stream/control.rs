use crate::core::error::EngineError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlowWindow {
    capacity: u32,
    available: u32,
}

impl FlowWindow {
    pub fn new(capacity: u32) -> Result<Self, EngineError> {
        if capacity == 0 {
            return Err(EngineError::invalid_input("flow window capacity must be positive"));
        }
        Ok(Self {
            capacity,
            available: capacity,
        })
    }

    pub fn available(&self) -> u32 {
        self.available
    }

    pub fn consume(&mut self, units: u32) -> Result<(), EngineError> {
        if units > self.available {
            return Err(EngineError::budget("stream flow window exhausted"));
        }
        self.available -= units;
        Ok(())
    }

    pub fn replenish(&mut self, units: u32) {
        self.available = self.available.saturating_add(units).min(self.capacity);
    }
}
