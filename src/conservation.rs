//! Conservation violation detection: charge drift = non-coordination-free behavior.

use crate::agent::{AgentId, AgentState, MultiAgentUpdate, WorldState};
use crate::noether::NoetherCharge;

/// A detected conservation violation.
#[derive(Debug, Clone)]
pub struct ConservationViolation {
    pub round: usize,
    pub expected_charge: f64,
    pub actual_charge: f64,
    pub drift: f64,
    pub severity: ViolationSeverity,
}

/// Severity of a conservation violation.
#[derive(Debug, Clone, PartialEq)]
pub enum ViolationSeverity {
    Negligible,
    Minor,
    Major,
    Critical,
}

impl std::fmt::Display for ViolationSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViolationSeverity::Negligible => write!(f, "NEGLIGIBLE"),
            ViolationSeverity::Minor => write!(f, "MINOR"),
            ViolationSeverity::Major => write!(f, "MAJOR"),
            ViolationSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

impl std::fmt::Display for ConservationViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Violation(round={}, drift={:.6}, severity={})",
            self.round, self.drift, self.severity
        )
    }
}

/// Result of a conservation analysis.
#[derive(Debug, Clone)]
pub struct ConservationResult {
    pub violations: Vec<ConservationViolation>,
    pub max_drift: f64,
    pub total_drift: f64,
    pub is_conserved: bool,
}

impl std::fmt::Display for ConservationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ConservationResult {{ conserved: {}, violations: {}, max_drift: {:.6} }}",
            self.is_conserved,
            self.violations.len(),
            self.max_drift
        )
    }
}

/// Detect conservation violations for a given charge and update rule.
pub fn detect_violations(
    charge: &NoetherCharge,
    rule: fn(&AgentId, &AgentState, &WorldState) -> AgentState,
    ws: &WorldState,
    rounds: usize,
    tolerance: f64,
) -> ConservationResult {
    let update = MultiAgentUpdate::new(rule);
    let mut current = ws.clone();
    let initial_charge = charge.value(&current);
    let mut violations = Vec::new();
    let mut max_drift = 0.0_f64;
    let mut total_drift = 0.0_f64;

    for round in 0..rounds {
        current = update.apply(&current);
        let actual_charge = charge.value(&current);
        let drift = (actual_charge - initial_charge).abs();

        if drift > tolerance {
            let severity = classify_severity(drift, tolerance);
            violations.push(ConservationViolation {
                round: round + 1,
                expected_charge: initial_charge,
                actual_charge,
                drift,
                severity,
            });
        }

        max_drift = max_drift.max(drift);
        total_drift += drift;
    }

    ConservationResult {
        is_conserved: violations.is_empty(),
        violations,
        max_drift,
        total_drift,
    }
}

/// Monitor charge drift over time and report when it exceeds thresholds.
pub fn monitor_charge(
    charge: &NoetherCharge,
    rule: fn(&AgentId, &AgentState, &WorldState) -> AgentState,
    ws: &WorldState,
    rounds: usize,
    minor_threshold: f64,
    major_threshold: f64,
    critical_threshold: f64,
) -> Vec<ConservationViolation> {
    let update = MultiAgentUpdate::new(rule);
    let mut current = ws.clone();
    let initial = charge.value(&current);
    let mut result = Vec::new();

    for round in 0..rounds {
        current = update.apply(&current);
        let actual = charge.value(&current);
        let drift = (actual - initial).abs();

        if drift > minor_threshold {
            let severity = if drift > critical_threshold {
                ViolationSeverity::Critical
            } else if drift > major_threshold {
                ViolationSeverity::Major
            } else {
                ViolationSeverity::Minor
            };

            result.push(ConservationViolation {
                round: round + 1,
                expected_charge: initial,
                actual_charge: actual,
                drift,
                severity,
            });
        }
    }

    result
}

/// Compute the charge trajectory for drift analysis.
pub fn charge_trajectory(
    charge: &NoetherCharge,
    rule: fn(&AgentId, &AgentState, &WorldState) -> AgentState,
    ws: &WorldState,
    rounds: usize,
) -> Vec<f64> {
    let update = MultiAgentUpdate::new(rule);
    let mut current = ws.clone();
    let mut trajectory = vec![charge.value(&current)];

    for _ in 0..rounds {
        current = update.apply(&current);
        trajectory.push(charge.value(&current));
    }

    trajectory
}

/// Detect if a rule is non-coordination-free by measuring charge drift
/// across multiple random initial conditions.
pub fn detect_non_calm(
    charge: &NoetherCharge,
    rule: fn(&AgentId, &AgentState, &WorldState) -> AgentState,
    num_agents: usize,
    rounds: usize,
    trials: usize,
    tolerance: f64,
) -> bool {
    use rand::Rng;
    let mut rng = rand::rng();

    for _ in 0..trials {
        let mut ws = WorldState::new();
        for i in 0..num_agents {
            let val: f64 = rng.random_range(0.0..100.0);
            ws.agents.insert(
                AgentId::new(format!("agent_{}", i)),
                AgentState::scalar(val),
            );
        }

        let result = detect_violations(charge, rule, &ws, rounds, tolerance);
        if !result.is_conserved {
            return true;
        }
    }
    false
}

fn classify_severity(drift: f64, tolerance: f64) -> ViolationSeverity {
    if drift > tolerance * 100.0 {
        ViolationSeverity::Critical
    } else if drift > tolerance * 10.0 {
        ViolationSeverity::Major
    } else {
        ViolationSeverity::Minor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sum_charge() -> NoetherCharge {
        NoetherCharge::new("sum", |ws| {
            ws.agents.values().flat_map(|s| s.values.iter()).sum()
        })
    }

    fn max_charge() -> NoetherCharge {
        NoetherCharge::new("max", |ws| {
            ws.agents.values().flat_map(|s| s.values.iter()).cloned().fold(f64::NEG_INFINITY, f64::max)
        })
    }

    fn sample_world() -> WorldState {
        WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(1.0))
            .with(AgentId::new("b"), AgentState::scalar(2.0))
            .with(AgentId::new("c"), AgentState::scalar(3.0))
    }

    #[test]
    fn test_sum_conserved_by_averaging() {
        let charge = sum_charge();
        let result = detect_violations(&charge, crate::agent::averaging_rule, &sample_world(), 10, 1e-10);
        assert!(result.is_conserved);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_max_conserved_by_max_rule() {
        let charge = max_charge();
        let ws = sample_world();
        let result = detect_violations(&charge, crate::agent::max_rule, &ws, 10, 1e-10);
        // Max rule: max stays at 3.0, so charge is conserved
        assert!(result.is_conserved);
    }

    #[test]
    fn test_violation_display() {
        let v = ConservationViolation {
            round: 3,
            expected_charge: 6.0,
            actual_charge: 5.0,
            drift: 1.0,
            severity: ViolationSeverity::Minor,
        };
        assert!(v.to_string().contains("round=3"));
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(ViolationSeverity::Critical.to_string(), "CRITICAL");
        assert_eq!(ViolationSeverity::Minor.to_string(), "MINOR");
    }

    #[test]
    fn test_conservation_result_display() {
        let r = ConservationResult {
            violations: vec![],
            max_drift: 0.0,
            total_drift: 0.0,
            is_conserved: true,
        };
        assert!(r.to_string().contains("conserved: true"));
    }

    #[test]
    fn test_charge_trajectory() {
        let charge = sum_charge();
        let traj = charge_trajectory(&charge, crate::agent::averaging_rule, &sample_world(), 5);
        assert_eq!(traj.len(), 6);
        // Sum should be constant at 6.0
        for v in &traj {
            assert!((v - 6.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_monitor_charge_no_violations() {
        let charge = sum_charge();
        let violations = monitor_charge(
            &charge,
            crate::agent::averaging_rule,
            &sample_world(),
            10,
            0.01,
            0.1,
            1.0,
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn test_detect_violations_max_drift() {
        let charge = sum_charge();
        let ws = sample_world();
        let result = detect_violations(&charge, crate::agent::averaging_rule, &ws, 10, 1e-10);
        assert!(result.max_drift < 1e-8);
    }
}
