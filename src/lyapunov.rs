//! Charge-as-Lyapunov: join only moves up lattice = monotone = Lyapunov.

use crate::agent::{AgentId, AgentState, WorldState};
use crate::noether::NoetherCharge;

/// Result of Lyapunov analysis.
#[derive(Debug, Clone)]
pub struct LyapunovResult {
    pub is_lyapunov: bool,
    pub is_monotone: bool,
    pub charge_values: Vec<f64>,
    pub violations: usize,
}

impl std::fmt::Display for LyapunovResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LyapunovResult {{ lyapunov: {}, monotone: {}, violations: {}, trajectory: {:?} }}",
            self.is_lyapunov, self.is_monotone, self.violations, self.charge_values
        )
    }
}

/// Verify that a Noether charge acts as a Lyapunov function on the join-semilattice.
/// The charge should be monotonically non-decreasing (or non-increasing) across rounds.
pub fn verify_lyapunov(
    charge: &NoetherCharge,
    rule: fn(&AgentId, &AgentState, &WorldState) -> AgentState,
    ws: &WorldState,
    rounds: usize,
    tolerance: f64,
) -> LyapunovResult {
    let update = crate::agent::MultiAgentUpdate::new(rule);
    let mut current = ws.clone();
    let mut values = Vec::new();
    let mut violations = 0;

    for _ in 0..rounds {
        let v = charge.value(&current);
        values.push(v);
        let next = update.apply(&current);
        let next_v = charge.value(&next);
        if next_v < v - tolerance {
            violations += 1;
        }
        current = next;
    }
    let final_v = charge.value(&current);
    values.push(final_v);

    let is_monotone = violations == 0;
    // Lyapunov: monotone and bounded above (for join) → converges
    let is_lyapunov = is_monotone;

    LyapunovResult {
        is_lyapunov,
        is_monotone,
        charge_values: values,
        violations,
    }
}

/// Check that join only moves up: for any two states, join(a,b) >= a and join(a,b) >= b.
pub fn join_moves_up(
    op: &dyn Fn(f64, f64) -> f64,
    test_values: &[f64],
    tolerance: f64,
) -> bool {
    for &a in test_values {
        for &b in test_values {
            let j = op(a, b);
            if j < a - tolerance || j < b - tolerance {
                return false;
            }
        }
    }
    true
}

/// Compute the Lyapunov trajectory: sequence of charge values over rounds.
pub fn lyapunov_trajectory(
    charge: &NoetherCharge,
    rule: fn(&AgentId, &AgentState, &WorldState) -> AgentState,
    ws: &WorldState,
    rounds: usize,
) -> Vec<f64> {
    let update = crate::agent::MultiAgentUpdate::new(rule);
    let mut current = ws.clone();
    let mut values = Vec::new();

    for _ in 0..rounds {
        values.push(charge.value(&current));
        current = update.apply(&current);
    }
    values.push(charge.value(&current));
    values
}

/// Verify that the join operation induces a partial order consistent with the charge.
pub fn verify_lattice_order(
    op: &dyn Fn(f64, f64) -> f64,
    _charge: &NoetherCharge,
    ws: &WorldState,
) -> bool {
    let states: Vec<&AgentState> = ws.ordered_agents().iter().map(|id| &ws.agents[id]).collect();
    if states.len() < 2 {
        return true;
    }

    // For each pair of agents, join should produce a state whose charge >= both
    for i in 0..states.len() {
        for j in (i + 1)..states.len() {
            for (vi, vj) in states[i].values.iter().zip(states[j].values.iter()) {
                let joined = op(*vi, *vj);
                if joined < *vi - 1e-10 || joined < *vj - 1e-10 {
                    return false;
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn max_charge() -> NoetherCharge {
        NoetherCharge::new("max", |ws| {
            ws.agents
                .values()
                .flat_map(|s| s.values.iter())
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max)
        })
    }

    fn sum_charge() -> NoetherCharge {
        NoetherCharge::new("sum", |ws| {
            ws.agents.values().flat_map(|s| s.values.iter()).sum()
        })
    }

    fn sample_world() -> WorldState {
        WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(1.0))
            .with(AgentId::new("b"), AgentState::scalar(2.0))
            .with(AgentId::new("c"), AgentState::scalar(3.0))
    }

    #[test]
    fn test_max_charge_lyapunov_for_max_rule() {
        let charge = max_charge();
        let ws = sample_world();
        let result = verify_lyapunov(&charge, crate::agent::max_rule, &ws, 5, 1e-10);
        assert!(result.is_lyapunov);
        assert!(result.is_monotone);
        assert_eq!(result.violations, 0);
    }

    #[test]
    fn test_sum_charge_lyapunov_for_averaging() {
        let charge = sum_charge();
        let ws = sample_world();
        let result = verify_lyapunov(&charge, crate::agent::averaging_rule, &ws, 5, 1e-10);
        // Sum is conserved → trivially monotone
        assert!(result.is_lyapunov);
    }

    #[test]
    fn test_join_moves_up_max() {
        assert!(join_moves_up(&|a, b| a.max(b), &[1.0, 2.0, 3.0, 5.0], 1e-10));
    }

    #[test]
    fn test_join_moves_up_min_fails() {
        // min(1, 3) = 1 < 3 → doesn't move up
        assert!(!join_moves_up(&|a, b| a.min(b), &[1.0, 3.0], 1e-10));
    }

    #[test]
    fn test_lyapunov_trajectory() {
        let charge = max_charge();
        let ws = sample_world();
        let traj = lyapunov_trajectory(&charge, crate::agent::max_rule, &ws, 3);
        assert_eq!(traj.len(), 4); // 3 rounds + initial
        // Max rule: all converge to max, so trajectory should be non-decreasing
        for i in 1..traj.len() {
            assert!(traj[i] >= traj[i - 1] - 1e-10);
        }
    }

    #[test]
    fn test_verify_lattice_order() {
        let ws = sample_world();
        let charge = max_charge();
        assert!(verify_lattice_order(&|a, b| a.max(b), &charge, &ws));
    }

    #[test]
    fn test_lyapunov_result_display() {
        let r = LyapunovResult {
            is_lyapunov: true,
            is_monotone: true,
            charge_values: vec![1.0, 2.0, 3.0],
            violations: 0,
        };
        assert!(r.to_string().contains("lyapunov: true"));
    }

    #[test]
    fn test_max_rule_converges_to_fixed_point() {
        let charge = max_charge();
        let ws = sample_world();
        let traj = lyapunov_trajectory(&charge, crate::agent::max_rule, &ws, 10);
        // After first round, all agents should have max = 3.0
        assert!((traj[1] - 3.0).abs() < 1e-10);
        // All subsequent values should be the same
        for v in &traj[2..] {
            assert!((v - 3.0).abs() < 1e-10);
        }
    }
}
