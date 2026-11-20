use crate::context::machine::CtxMachine;
use crate::core::error::EngineError;
use crate::core::ids::{ConvId, UserId};
use crate::core::traits::ConvStore;

pub fn append_message<S: ConvStore>(
    store: &S,
    machine: &mut CtxMachine,
    user: &UserId,
    conv: &ConvId,
    payload: &[u8],
    now_ms: i64,
) -> Result<u64, EngineError> {
    if payload.is_empty() {
        return Err(EngineError::invalid_input("message payload cannot be empty"));
    }
    let seq = store.append(user, conv, payload, now_ms)?;
    machine.commit_seq(seq)?;
    Ok(seq)
}
