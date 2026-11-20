use crate::core::error::EngineError;
use crate::stream::cancel::CancelFlag;
use crate::stream::control::FlowWindow;
use crate::wire::code::FrameKind;
use crate::wire::frame::Frame;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PipeStats {
    pub frames_forwarded: u64,
    pub bytes_forwarded: u64,
}

pub fn forward_delta(
    stream_id: u64,
    chunk: &[u8],
    window: &mut FlowWindow,
    cancel: &CancelFlag,
    stats: &mut PipeStats,
) -> Result<Frame, EngineError> {
    if cancel.is_canceled() {
        return Err(EngineError::canceled("stream canceled"));
    }
    if chunk.is_empty() {
        return Err(EngineError::invalid_input("empty stream chunk"));
    }
    window.consume(1)?;
    let frame = Frame::new(FrameKind::Delta, stream_id, 0, chunk.to_vec())?;
    stats.frames_forwarded = stats.frames_forwarded.saturating_add(1);
    stats.bytes_forwarded = stats.bytes_forwarded.saturating_add(chunk.len() as u64);
    Ok(frame)
}
