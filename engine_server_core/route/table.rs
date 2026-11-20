use crate::route::model::RouteTarget;

#[derive(Clone, Debug, Default)]
pub struct RouteTable {
    targets: Vec<RouteTarget>,
}

impl RouteTable {
    pub fn new(targets: Vec<RouteTarget>) -> Self {
        Self { targets }
    }

    pub fn match_model(&self, model: &str) -> Vec<&RouteTarget> {
        let trimmed = model.trim();
        let mut matched: Vec<&RouteTarget> = self
            .targets
            .iter()
            .filter(|t| t.model_prefix == "*" || trimmed.starts_with(&t.model_prefix))
            .collect();
        matched.sort_by(|a, b| {
            b.model_prefix
                .len()
                .cmp(&a.model_prefix.len())
                .then_with(|| a.priority.cmp(&b.priority))
        });
        matched
    }
}
