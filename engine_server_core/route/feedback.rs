use crate::route::model::EndpointHealth;

const CIRCUIT_FAILURE_THRESHOLD: u32 = 3;

pub fn record_success(health: &mut EndpointHealth, latency_ms: u32) {
    health.healthy = true;
    health.consecutive_failures = 0;
    health.ewma_latency_ms = ((health.ewma_latency_ms as u64 * 3 + latency_ms as u64) / 4) as u32;
}

pub fn record_failure(health: &mut EndpointHealth) {
    health.consecutive_failures = health.consecutive_failures.saturating_add(1);
    if health.consecutive_failures >= CIRCUIT_FAILURE_THRESHOLD {
        health.healthy = false;
    }
}
