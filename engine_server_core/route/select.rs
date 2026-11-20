use std::collections::HashMap;

use crate::core::error::EngineError;
use crate::core::ids::EndpointId;
use crate::route::model::{EndpointHealth, RouteTarget};
use crate::route::table::RouteTable;

pub fn select_endpoint(
    table: &RouteTable,
    health_map: &HashMap<EndpointId, EndpointHealth>,
    model: &str,
) -> Result<EndpointId, EngineError> {
    let candidates = table.match_model(model);
    if candidates.is_empty() {
        return Err(EngineError::not_found("no route configured for model"));
    }

    let mut best: Option<(&RouteTarget, u64)> = None;
    for target in candidates {
        let health = health_map
            .get(&target.endpoint_id)
            .copied()
            .unwrap_or_default();
        if !health.healthy {
            continue;
        }
        let load_penalty = (health.inflight as u64).saturating_mul(25);
        let weight_bonus = (target.weight.max(1) as u64).min(100);
        let score = (target.priority as u64)
            .saturating_mul(10_000)
            .saturating_add(health.ewma_latency_ms as u64)
            .saturating_add(load_penalty)
            .saturating_sub(weight_bonus);

        match best {
            None => best = Some((target, score)),
            Some((_, best_score)) if score < best_score => best = Some((target, score)),
            _ => {}
        }
    }

    best.map(|(t, _)| t.endpoint_id.clone())
        .ok_or_else(|| EngineError::budget("no healthy endpoint available"))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::route::feedback::{record_failure, record_success};

    #[test]
    fn route_selects_healthy_lowest_score_and_trips_circuit() {
        let ep1 = EndpointId::new("ep_fast");
        let ep2 = EndpointId::new("ep_backup");
        let table = RouteTable::new(vec![
            RouteTarget {
                endpoint_id: ep1.clone(),
                model_prefix: String::from("claude-"),
                priority: 1,
                weight: 50,
            },
            RouteTarget {
                endpoint_id: ep2.clone(),
                model_prefix: String::from("claude-"),
                priority: 2,
                weight: 50,
            },
        ]);

        let mut health = HashMap::new();
        health.insert(ep1.clone(), EndpointHealth::default());
        health.insert(ep2.clone(), EndpointHealth::default());

        assert_eq!(select_endpoint(&table, &health, "claude-sonnet").expect("ep1"), ep1);

        let h1 = health.get_mut(&ep1).expect("h1");
        record_failure(h1);
        record_failure(h1);
        record_failure(h1);
        assert_eq!(select_endpoint(&table, &health, "claude-sonnet").expect("ep2"), ep2);

        record_success(health.get_mut(&ep1).expect("h1"), 20);
        assert_eq!(select_endpoint(&table, &health, "claude-sonnet").expect("ep1 back"), ep1);
    }
}
