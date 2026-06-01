//! ACI verification: Associative + Commutative + Idempotent = join-semilattice.


/// Result of an ACI verification.
#[derive(Debug, Clone)]
pub struct AciResult {
    pub is_associative: bool,
    pub is_commutative: bool,
    pub is_idempotent: bool,
    pub is_join_semilattice: bool,
}

impl std::fmt::Display for AciResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AciResult {{ assoc: {}, comm: {}, idem: {}, join-semilattice: {} }}",
            self.is_associative, self.is_commutative, self.is_idempotent, self.is_join_semilattice
        )
    }
}

/// Verify ACI properties for a binary operation on f64.
pub fn verify_aci(
    op: &dyn Fn(f64, f64) -> f64,
    test_values: &[f64],
) -> AciResult {
    let is_commutative = check_commutativity(op, test_values);
    let is_associative = check_associativity(op, test_values);
    let is_idempotent = check_idempotency(op, test_values);

    AciResult {
        is_associative,
        is_commutative,
        is_idempotent,
        is_join_semilattice: is_associative && is_commutative && is_idempotent,
    }
}

fn check_commutativity(op: &dyn Fn(f64, f64) -> f64, values: &[f64]) -> bool {
    for &a in values {
        for &b in values {
            if (op(a, b) - op(b, a)).abs() > 1e-10 {
                return false;
            }
        }
    }
    true
}

fn check_associativity(op: &dyn Fn(f64, f64) -> f64, values: &[f64]) -> bool {
    for &a in values {
        for &b in values {
            for &c in values {
                let left = op(op(a, b), c);
                let right = op(a, op(b, c));
                if (left - right).abs() > 1e-10 {
                    return false;
                }
            }
        }
    }
    true
}

fn check_idempotency(op: &dyn Fn(f64, f64) -> f64, values: &[f64]) -> bool {
    for &a in values {
        if (op(a, a) - a).abs() > 1e-10 {
            return false;
        }
    }
    true
}

/// Verify ACI for a multi-agent aggregation function.
pub fn verify_aci_aggregate(
    aggregate: &dyn Fn(&[f64]) -> f64,
    test_inputs: &[Vec<f64>],
) -> AciResult {
    // Commutativity: aggregate(perm(xs)) == aggregate(xs)
    let is_commutative = test_inputs.iter().all(|xs| {
        if xs.len() < 2 {
            return true;
        }
        let base = aggregate(xs);
        // Test reverse
        let reversed: Vec<f64> = xs.iter().rev().copied().collect();
        if (aggregate(&reversed) - base).abs() > 1e-10 {
            return false;
        }
        true
    });

    // Associativity: aggregate([aggregate(sub), rest]) == aggregate(all)
    // Approximate: aggregate(a,b,c) ≈ aggregate(aggregate(a,b), c)
    let is_associative = test_inputs.iter().all(|xs| {
        if xs.len() < 3 {
            return true;
        }
        let full = aggregate(xs);
        // Take first two as sub-group
        let sub = aggregate(&xs[0..2]);
        let rebuilt = {
            let mut v = vec![sub];
            v.extend_from_slice(&xs[2..]);
            v
        };
        (aggregate(&rebuilt) - full).abs() < 1e-8
    });

    // Idempotency: aggregate([x, x, ...]) == x
    let is_idempotent = test_inputs.iter().all(|xs| {
        if xs.is_empty() {
            return true;
        }
        let x = xs[0];
        let duplicated = vec![x; xs.len().max(3)];
        (aggregate(&duplicated) - x).abs() < 1e-10
    });

    AciResult {
        is_associative,
        is_commutative,
        is_idempotent,
        is_join_semilattice: is_associative && is_commutative && is_idempotent,
    }
}

/// Compute the join (least upper bound) of multiple values using an ACI operation.
pub fn join_reduce(op: &dyn Fn(f64, f64) -> f64, values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values[1..].iter().fold(values[0], |acc, &v| op(acc, v))
}

/// Build a join-semilattice from an ACI operation and test values.
/// Returns the ordering relation (partial order induced by the join).
pub fn semilattice_order(
    op: &dyn Fn(f64, f64) -> f64,
    values: &[f64],
) -> Vec<(f64, f64)> {
    let mut order = Vec::new();
    for &a in values {
        for &b in values {
            // a ≤ b iff a ∨ b = b
            if (op(a, b) - b).abs() < 1e-10 {
                order.push((a, b));
            }
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_vals() -> Vec<f64> {
        vec![0.0, 1.0, 2.0, 3.0, 5.0]
    }

    #[test]
    fn test_max_is_aci() {
        let result = verify_aci(&|a, b| a.max(b), &test_vals());
        assert!(result.is_join_semilattice);
        assert!(result.is_associative);
        assert!(result.is_commutative);
        assert!(result.is_idempotent);
    }

    #[test]
    fn test_min_is_aci() {
        let result = verify_aci(&|a, b| a.min(b), &test_vals());
        assert!(result.is_join_semilattice);
    }

    #[test]
    fn test_sum_not_idempotent() {
        let result = verify_aci(&|a, b| a + b, &test_vals());
        assert!(!result.is_idempotent);
        assert!(result.is_associative);
        assert!(result.is_commutative);
        assert!(!result.is_join_semilattice);
    }

    #[test]
    fn test_subtraction_not_commutative() {
        let result = verify_aci(&|a, b| a - b, &test_vals());
        assert!(!result.is_commutative);
    }

    #[test]
    fn test_join_reduce_max() {
        let result = join_reduce(&|a, b| a.max(b), &[1.0, 3.0, 2.0, 5.0, 4.0]);
        assert!((result - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_join_reduce_min() {
        let result = join_reduce(&|a, b| a.min(b), &[3.0, 1.0, 2.0]);
        assert!((result - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_semilattice_order_max() {
        let order = semilattice_order(&|a, b| a.max(b), &[1.0, 2.0, 3.0]);
        // 1 ≤ 1, 1 ≤ 2, 1 ≤ 3, 2 ≤ 2, 2 ≤ 3, 3 ≤ 3
        assert!(order.contains(&(1.0, 2.0)));
        assert!(order.contains(&(1.0, 3.0)));
        assert!(order.contains(&(2.0, 3.0)));
        assert!(!order.contains(&(3.0, 1.0)));
    }

    #[test]
    fn test_aci_result_display() {
        let r = AciResult {
            is_associative: true,
            is_commutative: true,
            is_idempotent: true,
            is_join_semilattice: true,
        };
        assert!(r.to_string().contains("join-semilattice: true"));
    }

    #[test]
    fn test_verify_aci_aggregate_max() {
        let vals = vec![
            vec![1.0, 2.0, 3.0],
            vec![5.0, 1.0, 3.0],
        ];
        let result = verify_aci_aggregate(&|xs: &[f64]| xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max), &vals);
        assert!(result.is_join_semilattice);
    }

    #[test]
    fn test_verify_aci_aggregate_sum_not_idempotent() {
        let vals = vec![vec![1.0, 2.0, 3.0]];
        let result = verify_aci_aggregate(&|xs: &[f64]| xs.iter().sum(), &vals);
        assert!(!result.is_join_semilattice);
        assert!(!result.is_idempotent);
    }

    #[test]
    fn test_join_reduce_empty() {
        let result = join_reduce(&|a, b| a.max(b), &[]);
        assert!((result).abs() < 1e-10);
    }

    #[test]
    fn test_join_reduce_single() {
        let result = join_reduce(&|a, b| a.max(b), &[42.0]);
        assert!((result - 42.0).abs() < 1e-10);
    }
}
