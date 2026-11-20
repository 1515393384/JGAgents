use crate::core::ids::EndpointId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteTarget {
    pub endpoint_id: EndpointId,
    pub model_prefix: String,
    pub priority: u16,
    pub weight: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EndpointHealth {
    pub healthy: bool,
    pub ewma_latency_ms: u32,
    pub inflight: u32,
    pub consecutive_failures: u32,
}

impl Default for EndpointHealth {
    fn default() -> Self {
        Self {
            healthy: true,
            ewma_latency_ms: 50,
            inflight: 0,
            consecutive_failures: 0,
        }
    }
}
