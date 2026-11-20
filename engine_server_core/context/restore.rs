use crate::core::error::EngineError;
use crate::core::ids::{ConvId, UserId};
use crate::core::traits::ConvStore;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoredConv {
    pub last_seq: u64,
    pub rows: Vec<(u64, Vec<u8>)>,
}

pub fn restore_since<S: ConvStore>(
    store: &S,
    user: &UserId,
    conv: &ConvId,
    since_seq: u64,
) -> Result<RestoredConv, EngineError> {
    let rows = store.load_since(user, conv, since_seq)?;
    let last_seq = rows.last().map(|(seq, _)| *seq).unwrap_or(since_seq);
    Ok(RestoredConv { last_seq, rows })
}
