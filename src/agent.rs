//! Multi-agent state and learning rules.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Agent identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

/// A scalar state value for an agent.
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AgentState {
    pub values: Vec<f64>,
}

impl AgentState {
    pub fn new(values: Vec<f64>) -> Self {
        Self { values }
    }

    pub fn scalar(v: f64) -> Self {
        Self { values: vec![v] }
    }

    pub fn zero(dim: usize) -> Self {
        Self { values: vec![0.0; dim] }
    }

    pub fn dim(&self) -> usize {
        self.values.len()
    }

    pub fn as_slice(&self) -> &[f64] {
        &self.values
    }

    pub fn to_vec(&self) -> Vec<f64> {
        self.values.clone()
    }
}

/// A snapshot of all agents' states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldState {
    pub agents: HashMap<AgentId, AgentState>,
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    pub fn with(mut self, id: AgentId, state: AgentState) -> Self {
        self.agents.insert(id, state);
        self
    }

    pub fn get(&self, id: &AgentId) -> Option<&AgentState> {
        self.agents.get(id)
    }

    pub fn agent_ids(&self) -> Vec<&AgentId> {
        self.agents.keys().collect()
    }

    pub fn len(&self) -> usize {
        self.agents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// Return states as an ordered vector (sorted by agent id for determinism).
    pub fn ordered_states(&self) -> Vec<(&AgentId, &AgentState)> {
        let mut v: Vec<_> = self.agents.iter().collect();
        v.sort_by_key(|(id, _)| *id);
        v
    }

    /// Apply a permutation to agent IDs: returns a new WorldState where agent i
    /// gets the state of agent perm[i].
    pub fn permute(&self, perm: &[usize]) -> WorldState {
        let ids: Vec<&AgentId> = self.ordered_agents();
        let states: Vec<&AgentState> = ids.iter().map(|id| &self.agents[id]).collect();
        let mut new_ws = WorldState::new();
        for (target_idx, &source_idx) in perm.iter().enumerate() {
            let target_id = ids[target_idx].clone();
            let source_state = states[source_idx].clone();
            new_ws.agents.insert(target_id, source_state);
        }
        new_ws
    }

    /// Ordered agent IDs.
    pub fn ordered_agents(&self) -> Vec<&AgentId> {
        let mut v: Vec<_> = self.agent_ids();
        v.sort();
        v
    }
}

/// A learning rule: maps (agent_id, own_state, all_states) -> new_state.
pub type LearningRule = Box<dyn Fn(&AgentId, &AgentState, &WorldState) -> AgentState + Send + Sync>;

/// A simpler function-based rule on scalar states.
pub type ScalarRule = Box<dyn Fn(f64, &[f64]) -> f64 + Send + Sync>;

/// Multi-agent update: maps entire WorldState -> WorldState.
#[derive(Clone)]
pub struct MultiAgentUpdate {
    pub rule: fn(&AgentId, &AgentState, &WorldState) -> AgentState,
}

impl MultiAgentUpdate {
    pub fn new(rule: fn(&AgentId, &AgentState, &WorldState) -> AgentState) -> Self {
        Self { rule }
    }

    /// Apply the update to all agents simultaneously.
    pub fn apply(&self, ws: &WorldState) -> WorldState {
        let mut new_ws = WorldState::new();
        for id in ws.ordered_agents() {
            let state = &ws.agents[id];
            let new_state = (self.rule)(id, state, ws);
            new_ws.agents.insert(id.clone(), new_state);
        }
        new_ws
    }
}

/// A simple averaging rule (coordination-free).
pub fn averaging_rule(_id: &AgentId, _state: &AgentState, ws: &WorldState) -> AgentState {
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
}

/// A max rule (coordination-free, monotone).
pub fn max_rule(_id: &AgentId, _state: &AgentState, ws: &WorldState) -> AgentState {
    let states: Vec<&AgentState> = ws.agents.values().collect();
    if states.is_empty() {
        return AgentState::zero(1);
    }
    let dim = states[0].dim();
    let mut result = vec![f64::NEG_INFINITY; dim];
    for s in &states {
        for (i, v) in s.values.iter().enumerate() {
            result[i] = result[i].max(*v);
        }
    }
    AgentState::new(result)
}

/// A non-coordination-free rule: agent 0 gets a different update.
pub fn leader_rule(id: &AgentId, _state: &AgentState, ws: &WorldState) -> AgentState {
    let ids: Vec<&AgentId> = ws.ordered_agents();
    let first = ids.first().map(|x| *x);
    if Some(id) == first {
        // Leader gets max + bonus
        let states: Vec<&AgentState> = ws.agents.values().collect();
        let max_val = states.iter().flat_map(|s| s.values.iter()).cloned().fold(f64::NEG_INFINITY, f64::max);
        AgentState::scalar(max_val + 1.0)
    } else {
        averaging_rule(id, _state, ws)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_state_scalar() {
        let s = AgentState::scalar(3.14);
        assert_eq!(s.values, vec![3.14]);
        assert_eq!(s.dim(), 1);
    }

    #[test]
    fn test_world_state_with() {
        let ws = WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(1.0))
            .with(AgentId::new("b"), AgentState::scalar(2.0));
        assert_eq!(ws.len(), 2);
        assert_eq!(ws.get(&AgentId::new("a")).unwrap().values[0], 1.0);
    }

    #[test]
    fn test_world_state_ordered() {
        let ws = WorldState::new()
            .with(AgentId::new("c"), AgentState::scalar(3.0))
            .with(AgentId::new("a"), AgentState::scalar(1.0))
            .with(AgentId::new("b"), AgentState::scalar(2.0));
        let ordered: Vec<&str> = ws.ordered_agents().iter().map(|id| id.0.as_str()).collect();
        assert_eq!(ordered, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_averaging_rule() {
        let ws = WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(1.0))
            .with(AgentId::new("b"), AgentState::scalar(3.0));
        let result = averaging_rule(&AgentId::new("a"), &AgentState::scalar(1.0), &ws);
        assert!((result.values[0] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_max_rule() {
        let ws = WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(1.0))
            .with(AgentId::new("b"), AgentState::scalar(5.0))
            .with(AgentId::new("c"), AgentState::scalar(3.0));
        let result = max_rule(&AgentId::new("a"), &AgentState::scalar(1.0), &ws);
        assert!((result.values[0] - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_multi_agent_update_apply() {
        let update = MultiAgentUpdate::new(averaging_rule);
        let ws = WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(1.0))
            .with(AgentId::new("b"), AgentState::scalar(3.0));
        let new_ws = update.apply(&ws);
        assert!((new_ws.get(&AgentId::new("a")).unwrap().values[0] - 2.0).abs() < 1e-10);
        assert!((new_ws.get(&AgentId::new("b")).unwrap().values[0] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_world_state_permute() {
        let ws = WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(1.0))
            .with(AgentId::new("b"), AgentState::scalar(2.0))
            .with(AgentId::new("c"), AgentState::scalar(3.0));
        // Permutation: [2, 0, 1] means target 0 gets source 2, target 1 gets source 0, etc.
        let perm = vec![2, 0, 1];
        let permuted = ws.permute(&perm);
        assert_eq!(permuted.get(&AgentId::new("a")).unwrap().values[0], 3.0);
        assert_eq!(permuted.get(&AgentId::new("b")).unwrap().values[0], 1.0);
        assert_eq!(permuted.get(&AgentId::new("c")).unwrap().values[0], 2.0);
    }

    #[test]
    fn test_leader_rule_is_different() {
        let ws = WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(1.0))
            .with(AgentId::new("b"), AgentState::scalar(2.0));
        let leader_result = leader_rule(&AgentId::new("a"), &AgentState::scalar(1.0), &ws);
        let follower_result = leader_rule(&AgentId::new("b"), &AgentState::scalar(2.0), &ws);
        // Leader gets max + 1 = 3.0, follower gets average = 1.5
        assert!((leader_result.values[0] - 3.0).abs() < 1e-10);
        assert!((follower_result.values[0] - 1.5).abs() < 1e-10);
    }
}
