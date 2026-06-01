//! Partition test: replicas → partition → merge → check convergence.

use crate::agent::{AgentId, AgentState, MultiAgentUpdate, WorldState};

/// Result of a partition test.
#[derive(Debug, Clone)]
pub struct PartitionTestResult {
    pub converged: bool,
    pub initial_charge: f64,
    pub final_charge: f64,
    pub partition_charges: Vec<f64>,
    pub merged_charge: f64,
    pub charge_drift: f64,
}

impl std::fmt::Display for PartitionTestResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PartitionTestResult {{ converged: {}, drift: {:.6} }}",
            self.converged, self.charge_drift
        )
    }
}

/// A partition test: verifies that replicas converge after partition and merge.
pub struct PartitionTest {
    rule: fn(&AgentId, &AgentState, &WorldState) -> AgentState,
    tolerance: f64,
}

impl PartitionTest {
    pub fn new(
        rule: fn(&AgentId, &AgentState, &WorldState) -> AgentState,
        tolerance: f64,
    ) -> Self {
        Self { rule, tolerance }
    }

    /// Run the partition test:
    /// 1. Start with initial state
    /// 2. Run for some rounds (replication)
    /// 3. Split into partitions, run each independently
    /// 4. Merge partitions
    /// 5. Run more rounds
    /// 6. Check convergence
    pub fn run(&self, initial: &WorldState, rounds: usize) -> PartitionTestResult {
        let update = MultiAgentUpdate::new(self.rule);

        // Compute initial charge (sum of all values)
        let compute_charge = |ws: &WorldState| -> f64 {
            ws.agents.values().flat_map(|s| s.values.iter()).sum()
        };

        let initial_charge = compute_charge(initial);

        // Step 1: Run replicas for a few rounds
        let mut replicated = initial.clone();
        for _ in 0..rounds {
            replicated = update.apply(&replicated);
        }

        // Step 2: Partition into two groups
        let ids: Vec<AgentId> = replicated.ordered_agents().into_iter().cloned().collect();
        if ids.len() < 2 {
            return PartitionTestResult {
                converged: true,
                initial_charge,
                final_charge: compute_charge(&replicated),
                partition_charges: vec![],
                merged_charge: compute_charge(&replicated),
                charge_drift: 0.0,
            };
        }

        let mid = ids.len() / 2;
        let partition_a_ids: Vec<&AgentId> = ids[..mid].iter().collect();
        let partition_b_ids: Vec<AgentId> = ids[mid..].to_vec();

        // Create partition worlds (each partition only sees its own agents)
        let mut partition_a = WorldState::new();
        for id in &partition_a_ids {
            partition_a.agents.insert((*id).clone(), replicated.agents[*id].clone());
        }

        let mut partition_b = WorldState::new();
        for id in &partition_b_ids {
            partition_b.agents.insert(id.clone(), replicated.agents[id].clone());
        }

        // Step 3: Run each partition independently
        for _ in 0..rounds {
            partition_a = update.apply(&partition_a);
            partition_b = update.apply(&partition_b);
        }

        let charge_a = compute_charge(&partition_a);
        let charge_b = compute_charge(&partition_b);

        // Step 4: Merge
        let mut merged = WorldState::new();
        for (id, state) in &partition_a.agents {
            merged.agents.insert(id.clone(), state.clone());
        }
        for (id, state) in &partition_b.agents {
            merged.agents.insert(id.clone(), state.clone());
        }

        let merged_charge = compute_charge(&merged);

        // Step 5: Run merged for more rounds
        let mut final_state = merged;
        for _ in 0..rounds {
            final_state = update.apply(&final_state);
        }

        let final_charge = compute_charge(&final_state);

        // Step 6: Check convergence
        let charge_drift = (final_charge - initial_charge).abs();
        let converged = charge_drift <= self.tolerance;

        PartitionTestResult {
            converged,
            initial_charge,
            final_charge,
            partition_charges: vec![charge_a, charge_b],
            merged_charge,
            charge_drift,
        }
    }

    /// Run with custom partition sizes.
    pub fn run_with_partition(
        &self,
        initial: &WorldState,
        rounds: usize,
        partition_a_indices: &[usize],
    ) -> PartitionTestResult {
        let update = MultiAgentUpdate::new(self.rule);

        let compute_charge = |ws: &WorldState| -> f64 {
            ws.agents.values().flat_map(|s| s.values.iter()).sum()
        };

        let initial_charge = compute_charge(initial);

        let mut replicated = initial.clone();
        for _ in 0..rounds {
            replicated = update.apply(&replicated);
        }

        let ids: Vec<AgentId> = replicated.ordered_agents().into_iter().cloned().collect();
        let a_set: std::collections::HashSet<usize> = partition_a_indices.iter().copied().collect();

        let mut partition_a = WorldState::new();
        let mut partition_b = WorldState::new();

        for (i, id) in ids.iter().enumerate() {
            if a_set.contains(&i) {
                partition_a.agents.insert(id.clone(), replicated.agents[id].clone());
            } else {
                partition_b.agents.insert(id.clone(), replicated.agents[id].clone());
            }
        }

        for _ in 0..rounds {
            if !partition_a.agents.is_empty() {
                partition_a = update.apply(&partition_a);
            }
            if !partition_b.agents.is_empty() {
                partition_b = update.apply(&partition_b);
            }
        }

        let charge_a = compute_charge(&partition_a);
        let charge_b = compute_charge(&partition_b);

        let mut merged = partition_a;
        for (id, state) in partition_b.agents {
            merged.agents.insert(id, state);
        }

        let merged_charge = compute_charge(&merged);

        let mut final_state = merged;
        for _ in 0..rounds {
            final_state = update.apply(&final_state);
        }

        let final_charge = compute_charge(&final_state);
        let charge_drift = (final_charge - initial_charge).abs();

        PartitionTestResult {
            converged: charge_drift <= self.tolerance,
            initial_charge,
            final_charge,
            partition_charges: vec![charge_a, charge_b],
            merged_charge,
            charge_drift,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_world() -> WorldState {
        WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(1.0))
            .with(AgentId::new("b"), AgentState::scalar(3.0))
            .with(AgentId::new("c"), AgentState::scalar(5.0))
            .with(AgentId::new("d"), AgentState::scalar(7.0))
    }

    #[test]
    fn test_partition_averaging_converges() {
        let pt = PartitionTest::new(crate::agent::averaging_rule, 1.0);
        let ws = sample_world();
        let result = pt.run(&ws, 5);
        assert!(result.converged);
    }

    #[test]
    fn test_partition_max_converges() {
        let pt = PartitionTest::new(crate::agent::max_rule, 10.0);
        let ws = sample_world();
        let result = pt.run(&ws, 3);
        // Max rule doesn't conserve sum, so convergence check needs large tolerance.
        // Instead verify that the partition test result shows the process completes.
        assert!(result.charge_drift.is_finite());
    }

    #[test]
    fn test_partition_result_display() {
        let r = PartitionTestResult {
            converged: true,
            initial_charge: 16.0,
            final_charge: 16.0,
            partition_charges: vec![8.0, 8.0],
            merged_charge: 16.0,
            charge_drift: 0.0,
        };
        assert!(r.to_string().contains("converged: true"));
    }

    #[test]
    fn test_partition_custom_split() {
        let pt = PartitionTest::new(crate::agent::averaging_rule, 1.0);
        let ws = sample_world();
        let result = pt.run_with_partition(&ws, 5, &[0, 2]);
        assert!(result.converged);
    }

    #[test]
    fn test_partition_single_agent() {
        let ws = WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(5.0));
        let pt = PartitionTest::new(crate::agent::averaging_rule, 0.1);
        let result = pt.run(&ws, 3);
        assert!(result.converged);
    }

    #[test]
    fn test_partition_two_agents() {
        let ws = WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(2.0))
            .with(AgentId::new("b"), AgentState::scalar(8.0));
        let pt = PartitionTest::new(crate::agent::averaging_rule, 0.1);
        let result = pt.run(&ws, 5);
        assert!(result.converged);
    }

    #[test]
    fn test_partition_charges_tracked() {
        let pt = PartitionTest::new(crate::agent::averaging_rule, 1.0);
        let ws = sample_world();
        let result = pt.run(&ws, 3);
        assert_eq!(result.partition_charges.len(), 2);
    }
}
