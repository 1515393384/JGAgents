use crate::core::error::EngineError;
use crate::core::ids::{ConvId, EndpointId, UserId};
use crate::stream::model::{StreamDescriptor, StreamId, StreamState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenStreamReq {
    pub stream_id: StreamId,
    pub user_id: UserId,
    pub conv_id: ConvId,
    pub endpoint_id: EndpointId,
    pub model: String,
    pub now_ms: i64,
}

pub fn prepare_stream(req: &OpenStreamReq) -> Result<StreamDescriptor, EngineError> {
    let model = req.model.trim();
    if model.is_empty() {
        return Err(EngineError::invalid_input("model cannot be empty"));
    }
    Ok(StreamDescriptor {
        id: req.stream_id,
        state: StreamState::Opening,
        model: model.to_string(),
        opened_at_ms: req.now_ms,
    })
}
