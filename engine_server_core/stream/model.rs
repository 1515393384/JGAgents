#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StreamId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamState {
    Opening,
    Streaming,
    Paused,
    Canceled,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamDescriptor {
    pub id: StreamId,
    pub state: StreamState,
    pub model: String,
    pub opened_at_ms: i64,
}
