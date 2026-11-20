use crate::core::error::EngineError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CtxPhase {
    Idle,
    Streaming,
    Draining,
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CtxMachine {
    phase: CtxPhase,
    last_seq: u64,
}

impl Default for CtxMachine {
    fn default() -> Self {
        Self {
            phase: CtxPhase::Idle,
            last_seq: 0,
        }
    }
}

impl CtxMachine {
    pub fn phase(&self) -> CtxPhase {
        self.phase
    }

    pub fn last_seq(&self) -> u64 {
        self.last_seq
    }

    pub fn begin_stream(&mut self) -> Result<(), EngineError> {
        match self.phase {
            CtxPhase::Idle => {
                self.phase = CtxPhase::Streaming;
                Ok(())
            }
            _ => Err(EngineError::conflict("cannot start stream in current phase")),
        }
    }

    pub fn commit_seq(&mut self, seq: u64) -> Result<(), EngineError> {
        if self.phase == CtxPhase::Closed {
            return Err(EngineError::conflict("conversation is closed"));
        }
        if seq <= self.last_seq {
            return Err(EngineError::conflict("non-monotonic context seq"));
        }
        self.last_seq = seq;
        if self.phase == CtxPhase::Streaming {
            self.phase = CtxPhase::Idle;
        }
        Ok(())
    }

    pub fn drain(&mut self) {
        if self.phase != CtxPhase::Closed {
            self.phase = CtxPhase::Draining;
        }
    }

    pub fn close(&mut self) {
        self.phase = CtxPhase::Closed;
    }
}
