//! Noether charge computation: permutation-invariant aggregate from symmetry.

use crate::agent::{AgentId, AgentState, WorldState};
use crate::symmetry::SymmetryGroup;

/// A Noether charge: a permutation-invariant quantity conserved by the update rule.
#[derive(Debug, Clone)]
pub struct NoetherCharge {
    pub name: String,
    pub compute: fn(&WorldState) -> f64,
}

impl NoetherCharge {
    pub fn new(name: impl Into<String>, compute: fn(&WorldState) -> f64) -> Self {
        Self {
            name: name.into(),
            compute,
        }
    }

    /// Compute the charge for a world state.
    pub fn value(&self, ws: &WorldState) -> f64 {
        (self.compute)(ws)
    }

    /// Verify that this charge is permutation-invariant.
    pub fn is_permutation_invariant(&self, ws: &WorldState) -> bool {
        let base = self.value(ws);
        let n = ws.len();
        if n <= 1 {
            return true;
        }

        // Test a few permutations
        let perms = test_permutations(n);
        for perm in perms {
            let permuted = ws.permute(&perm);
            let val = self.value(&permuted);
            if (val - base).abs() > 1e-10 {
                return false;
            }
        }
        true
    }

    /// Check conservation: charge should be the same before and after an update.
    pub fn is_conserved(
        &self,
        rule: fn(&AgentId, &AgentState, &WorldState) -> AgentState,
        ws: &WorldState,
        rounds: usize,
        tolerance: f64,
    ) -> bool {
        let mut current = ws.clone();
        let initial_charge = self.value(&current);

        for _ in 0..rounds {
            let update = crate::agent::MultiAgentUpdate::new(rule);
            current = update.apply(&current);
            let charge = self.value(&current);
            if (charge - initial_charge).abs() > tolerance {
                return false;
            }
        }
        true
    }
}

/// Compute the Noether charge from a symmetry group.
/// The charge is the orbit-average of the identity function.
pub fn compute_charge_from_symmetry(
    group: &SymmetryGroup,
    ws: &WorldState,
) -> f64 {
    let base: f64 = ws.agents.values().map(|s| s.values.iter().sum::<f64>()).sum::<f64>();
    if group.generators.is_empty() {
        return base;
    }

    let _n = ws.len();
    let states: Vec<&AgentState> = ws.ordered_agents().iter().map(|id| &ws.agents[id]).collect();
    let ids: Vec<&AgentId> = ws.ordered_agents();

    // Average over all generated permutations
    let mut total = 0.0;
    let mut count = 0;

    for perm in &group.generators {
        let mut state_sum = 0.0;
        for (target_idx, &source_idx) in perm.iter().enumerate() {
            let _target_id = ids[target_idx];
            let source_state = &states[source_idx];
            // Sum: each target position gets the value from source
            state_sum += source_state.values.iter().sum::<f64>();
        }
        total += state_sum;
        count += 1;
    }

    if count > 0 {
        total / count as f64
    } else {
        base
    }
}

/// Find all Noether charges for a given update rule and world state.
/// Tests standard charges: sum, max, min, product.
pub fn find_charges(
    rule: fn(&AgentId, &AgentState, &WorldState) -> AgentState,
    ws: &WorldState,
    rounds: usize,
    tolerance: f64,
) -> Vec<NoetherCharge> {
    let candidates = vec![
        NoetherCharge::new("sum", |ws: &WorldState| {
            ws.agents.values().flat_map(|s| s.values.iter()).sum()
        }),
        NoetherCharge::new("max", |ws: &WorldState| {
            ws.agents
                .values()
                .flat_map(|s| s.values.iter())
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max)
        }),
        NoetherCharge::new("min", |ws: &WorldState| {
            ws.agents
                .values()
                .flat_map(|s| s.values.iter())
                .cloned()
                .fold(f64::INFINITY, f64::min)
        }),
        NoetherCharge::new("mean", |ws: &WorldState| {
            let vals: Vec<f64> = ws.agents.values().flat_map(|s| s.values.iter()).copied().collect();
            if vals.is_empty() { 0.0 } else { vals.iter().sum::<f64>() / vals.len() as f64 }
        }),
    ];

    candidates
        .into_iter()
        .filter(|c| c.is_conserved(rule, ws, rounds, tolerance))
        .collect()
}

fn test_permutations(n: usize) -> Vec<Vec<usize>> {
    if n <= 1 {
        return vec![(0..n).collect()];
    }

    let mut result = Vec::new();

    // Identity
    result.push((0..n).collect());

    // Swap first two
    let mut p: Vec<usize> = (0..n).collect();
    p.swap(0, 1);
    result.push(p);

    // Reverse
    let mut p: Vec<usize> = (0..n).collect();
    p.reverse();
    result.push(p);

    // Rotate
    if n >= 3 {
        let mut p: Vec<usize> = (0..n).collect();
        let first = p.remove(0);
        p.push(first);
        result.push(p);
    }

    result
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
    fn test_sum_charge() {
        let charge = NoetherCharge::new("sum", |ws| {
            ws.agents.values().flat_map(|s| s.values.iter()).sum()
        });
        let ws = sample_world();
        assert!((charge.value(&ws) - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_sum_charge_permutation_invariant() {
        let charge = NoetherCharge::new("sum", |ws| {
            ws.agents.values().flat_map(|s| s.values.iter()).sum()
        });
        let ws = sample_world();
        assert!(charge.is_permutation_invariant(&ws));
    }

    #[test]
    fn test_max_charge_permutation_invariant() {
        let charge = NoetherCharge::new("max", |ws| {
            ws.agents.values().flat_map(|s| s.values.iter()).cloned().fold(f64::NEG_INFINITY, f64::max)
        });
        let ws = sample_world();
        assert!(charge.is_permutation_invariant(&ws));
    }

    #[test]
    fn test_non_permutation_invariant_charge() {
        // First-agent charge
        let charge = NoetherCharge::new("first", |ws| {
            let ordered: Vec<_> = ws.ordered_agents();
            ordered.first().map(|id| ws.agents[*id].values[0]).unwrap_or(0.0)
        });
        let ws = sample_world();
        assert!(!charge.is_permutation_invariant(&ws));
    }

    #[test]
    fn test_sum_conserved_by_averaging() {
        let charge = NoetherCharge::new("sum", |ws| {
            ws.agents.values().flat_map(|s| s.values.iter()).sum()
        });
        let ws = sample_world();
        assert!(charge.is_conserved(crate::agent::averaging_rule, &ws, 3, 0.1));
    }

    #[test]
    fn test_find_charges_averaging() {
        let ws = sample_world();
        let charges = find_charges(crate::agent::averaging_rule, &ws, 3, 0.1);
        // Sum should be conserved, max/min might not be
        let names: Vec<&str> = charges.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"sum"));
    }

    #[test]
    fn test_compute_charge_from_symmetry() {
        let group = SymmetryGroup::full_symmetric(3);
        let ws = sample_world();
        let charge = compute_charge_from_symmetry(&group, &ws);
        // Should be close to the sum
        assert!(charge > 0.0);
    }

    #[test]
    fn test_charge_value_consistency() {
        let charge = NoetherCharge::new("sum", |ws| {
            ws.agents.values().flat_map(|s| s.values.iter()).sum()
        });
        let ws1 = WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(5.0))
            .with(AgentId::new("b"), AgentState::scalar(5.0));
        let ws2 = WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(10.0));
        assert!((charge.value(&ws1) - 10.0).abs() < 1e-10);
        assert!((charge.value(&ws2) - 10.0).abs() < 1e-10);
    }
}
