//! Coordination-free learning rule generator.
//! Given a symmetry group, produce valid coordination-free rules.

use crate::agent::{AgentId, AgentState, WorldState};
use crate::symmetry::SymmetryGroup;
use serde::{Deserialize, Serialize};

/// Type of aggregation for a coordination-free rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregationType {
    Max,
    Min,
    Mean,
    Median,
    GatedMean { gate_threshold: f64 },
    WeightedJoin { weights: Vec<f64> },
}

/// A generated coordination-free rule.
pub struct GeneratedRule {
    pub name: String,
    pub aggregation: AggregationType,
    pub update: fn(&AgentId, &AgentState, &WorldState) -> AgentState,
    pub update_boxed: Option<Box<dyn Fn(&AgentId, &AgentState, &WorldState) -> AgentState + Send + Sync>>,
}

impl GeneratedRule {
    fn new_simple(name: String, aggregation: AggregationType, update: fn(&AgentId, &AgentState, &WorldState) -> AgentState) -> Self {
        Self { name, aggregation, update, update_boxed: None }
    }

    fn new_boxed(name: String, aggregation: AggregationType, update_boxed: Box<dyn Fn(&AgentId, &AgentState, &WorldState) -> AgentState + Send + Sync>) -> Self {
        // Stub fn pointer for the boxed version
        Self {
            name,
            aggregation,
            update: |_, _, _| AgentState::zero(0),
            update_boxed: Some(update_boxed),
        }
    }

    pub fn apply(&self, id: &AgentId, state: &AgentState, ws: &WorldState) -> AgentState {
        if let Some(ref f) = self.update_boxed {
            f(id, state, ws)
        } else {
            (self.update)(id, state, ws)
        }
    }
}

/// Generate a coordination-free rule given a symmetry group.
/// The rule will be permutation-invariant by construction.
pub fn generate_rule(
    _group: &SymmetryGroup,
    aggregation: AggregationType,
) -> GeneratedRule {
    match aggregation {
        AggregationType::Max => GeneratedRule::new_simple(
            "max-join".to_string(),
            aggregation.clone(),
            |_, _, ws: &WorldState| {
                let _vals: Vec<f64> = ws.agents.values().flat_map(|s| s.values.iter()).copied().collect();
                let dim = ws.agents.values().next().map(|s| s.dim()).unwrap_or(1);
                let mut result = vec![f64::NEG_INFINITY; dim];
                for s in ws.agents.values() {
                    for (i, v) in s.values.iter().enumerate() {
                        result[i] = result[i].max(*v);
                    }
                }
                AgentState::new(result)
            },
        ),
        AggregationType::Min => GeneratedRule::new_simple(
            "min-meet".to_string(),
            aggregation.clone(),
            |_, _, ws: &WorldState| {
                let mut result = vec![f64::INFINITY; 1];
                for s in ws.agents.values() {
                    for (i, v) in s.values.iter().enumerate() {
                        if i >= result.len() {
                            result.push(f64::INFINITY);
                        }
                        result[i] = result[i].min(*v);
                    }
                }
                if result.is_empty() { result.push(0.0); }
                AgentState::new(result)
            },
        ),
        AggregationType::Mean => GeneratedRule::new_simple(
            "mean-average".to_string(),
            aggregation.clone(),
            |_, _, ws: &WorldState| {
                let states: Vec<&AgentState> = ws.agents.values().collect();
                if states.is_empty() {
                    return AgentState::zero(1);
                }
                let dim = states[0].dim();
                let mut sum = vec![0.0; dim];
                for s in &states {
                    for (i, v) in s.values.iter().enumerate() {
                        sum[i] += v;
                    }
                }
                let n = states.len() as f64;
                AgentState::new(sum.into_iter().map(|v| v / n).collect())
            },
        ),
        AggregationType::Median => GeneratedRule::new_simple(
            "median".to_string(),
            aggregation.clone(),
            |_, _, ws: &WorldState| {
                let states: Vec<&AgentState> = ws.agents.values().collect();
                if states.is_empty() {
                    return AgentState::zero(1);
                }
                let dim = states[0].dim();
                let mut result = Vec::new();
                for i in 0..dim {
                    let mut vals: Vec<f64> = states.iter().map(|s| s.values[i]).collect();
                    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    let mid = vals.len() / 2;
                    let median = if vals.len() % 2 == 0 {
                        (vals[mid - 1] + vals[mid]) / 2.0
                    } else {
                        vals[mid]
                    };
                    result.push(median);
                }
                AgentState::new(result)
            },
        ),
        AggregationType::GatedMean { gate_threshold } => {
            let threshold = gate_threshold;
            GeneratedRule::new_boxed(
                format!("gated-mean-{:.2}", gate_threshold),
                aggregation.clone(),
                Box::new(move |_, _, ws: &WorldState| {
                    let states: Vec<&AgentState> = ws.agents.values().collect();
                    if states.is_empty() {
                        return AgentState::zero(1);
                    }
                    let dim = states[0].dim();
                    let mut sum = vec![0.0; dim];
                    let mut count = 0usize;
                    for s in &states {
                        let mean_val: f64 = s.values.iter().sum::<f64>() / s.dim() as f64;
                        if mean_val >= threshold {
                            for (i, v) in s.values.iter().enumerate() {
                                sum[i] += v;
                            }
                            count += 1;
                        }
                    }
                    if count == 0 {
                        return AgentState::zero(dim);
                    }
                    let n = count as f64;
                    AgentState::new(sum.into_iter().map(|v| v / n).collect())
                }),
            )
        }
        AggregationType::WeightedJoin { .. } => GeneratedRule::new_simple(
            "weighted-join".to_string(),
            aggregation.clone(),
            |_, _, ws: &WorldState| {
                let vals: Vec<f64> = ws.agents.values().flat_map(|s| s.values.iter()).copied().collect();
                let max_val = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                AgentState::scalar(max_val)
            },
        ),
    }
}

/// Generate all standard coordination-free rules for a symmetry group.
pub fn generate_all_rules(group: &SymmetryGroup) -> Vec<GeneratedRule> {
    let aggregations = vec![
        AggregationType::Max,
        AggregationType::Min,
        AggregationType::Mean,
        AggregationType::Median,
        AggregationType::GatedMean { gate_threshold: 0.0 },
    ];

    aggregations.into_iter().map(|agg| generate_rule(group, agg)).collect()
}

/// Verify that a generated rule is coordination-free.
pub fn verify_generated_rule(rule: &GeneratedRule, ws: &WorldState, tolerance: f64) -> bool {
    let apply_all = |ws: &WorldState| -> WorldState {
        let mut new_ws = WorldState::new();
        for id in ws.ordered_agents() {
            let state = &ws.agents[id];
            let new_state = rule.apply(id, state, ws);
            new_ws.agents.insert(id.clone(), new_state);
        }
        new_ws
    };
    let base = apply_all(ws);

    // Test permutation invariance
    let n = ws.len();
    if n <= 1 {
        return true;
    }

    // Test with a swap permutation
    let swap_perm: Vec<usize> = {
        let mut p: Vec<usize> = (0..n).collect();
        if n >= 2 {
            p.swap(0, 1);
        }
        p
    };

    let permuted = ws.permute(&swap_perm);
    let permuted_result = apply_all(&permuted);

    // Results should be the same (all agents get the same state in coord-free rules)
    let ids: Vec<&AgentId> = ws.ordered_agents();
    for id in ids {
        let base_state = base.get(id);
        let perm_state = permuted_result.get(id);
        match (base_state, perm_state) {
            (Some(b), Some(p)) => {
                for (bv, pv) in b.values.iter().zip(p.values.iter()) {
                    if (bv - pv).abs() > tolerance {
                        return false;
                    }
                }
            }
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_world() -> WorldState {
        WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(1.0))
            .with(AgentId::new("b"), AgentState::scalar(2.0))
            .with(AgentId::new("c"), AgentState::scalar(3.0))
    }

    #[test]
    fn test_generate_max_rule() {
        let group = SymmetryGroup::full_symmetric(3);
        let rule = generate_rule(&group, AggregationType::Max);
        assert_eq!(rule.name, "max-join");

        let ws = sample_world();
        let update = |ws: &WorldState| -> WorldState {
            let mut new_ws = WorldState::new();
            for id in ws.ordered_agents() {
                let state = &ws.agents[id];
                let new_state = rule.apply(id, state, ws);
                new_ws.agents.insert(id.clone(), new_state);
            }
            new_ws
        };
        let result = update(&ws);
        // All agents should have max value 3.0
        for state in result.agents.values() {
            assert!((state.values[0] - 3.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_generate_min_rule() {
        let group = SymmetryGroup::full_symmetric(3);
        let rule = generate_rule(&group, AggregationType::Min);
        let ws = sample_world();
        let apply_all = |ws: &WorldState| -> WorldState {
            let mut new_ws = WorldState::new();
            for id in ws.ordered_agents() {
                let state = &ws.agents[id];
                let new_state = rule.apply(id, state, ws);
                new_ws.agents.insert(id.clone(), new_state);
            }
            new_ws
        };
        let result = apply_all(&ws);
        for state in result.agents.values() {
            assert!((state.values[0] - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_generate_mean_rule() {
        let group = SymmetryGroup::full_symmetric(3);
        let rule = generate_rule(&group, AggregationType::Mean);
        let ws = sample_world();
        let apply_all = |ws: &WorldState| -> WorldState {
            let mut new_ws = WorldState::new();
            for id in ws.ordered_agents() {
                let state = &ws.agents[id];
                let new_state = rule.apply(id, state, ws);
                new_ws.agents.insert(id.clone(), new_state);
            }
            new_ws
        };
        let result = apply_all(&ws);
        for state in result.agents.values() {
            assert!((state.values[0] - 2.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_generate_median_rule() {
        let group = SymmetryGroup::full_symmetric(3);
        let rule = generate_rule(&group, AggregationType::Median);
        let ws = sample_world();
        let apply_all = |ws: &WorldState| -> WorldState {
            let mut new_ws = WorldState::new();
            for id in ws.ordered_agents() {
                let state = &ws.agents[id];
                let new_state = rule.apply(id, state, ws);
                new_ws.agents.insert(id.clone(), new_state);
            }
            new_ws
        };
        let result = apply_all(&ws);
        // Median of [1, 2, 3] = 2.0
        for state in result.agents.values() {
            assert!((state.values[0] - 2.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_generate_all_rules() {
        let group = SymmetryGroup::full_symmetric(3);
        let rules = generate_all_rules(&group);
        assert_eq!(rules.len(), 5);
    }

    #[test]
    fn test_verify_generated_max() {
        let group = SymmetryGroup::full_symmetric(3);
        let rule = generate_rule(&group, AggregationType::Max);
        let ws = sample_world();
        assert!(verify_generated_rule(&rule, &ws, 1e-10));
    }

    #[test]
    fn test_verify_generated_mean() {
        let group = SymmetryGroup::full_symmetric(3);
        let rule = generate_rule(&group, AggregationType::Mean);
        let ws = sample_world();
        assert!(verify_generated_rule(&rule, &ws, 1e-10));
    }

    #[test]
    fn test_generate_gated_mean() {
        let group = SymmetryGroup::full_symmetric(3);
        let rule = generate_rule(&group, AggregationType::GatedMean { gate_threshold: 1.5 });
        let ws = sample_world();
        let apply_all = |ws: &WorldState| -> WorldState {
            let mut new_ws = WorldState::new();
            for id in ws.ordered_agents() {
                let state = &ws.agents[id];
                let new_state = rule.apply(id, state, ws);
                new_ws.agents.insert(id.clone(), new_state);
            }
            new_ws
        };
        let result = apply_all(&ws);
        // Only agents with mean >= 1.5: b(2.0) and c(3.0)
        // Mean of those: (2.0 + 3.0) / 2 = 2.5
        for state in result.agents.values() {
            assert!((state.values[0] - 2.5).abs() < 1e-10);
        }
    }
}
