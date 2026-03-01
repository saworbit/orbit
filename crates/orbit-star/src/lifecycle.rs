//! Star Lifecycle Hooks: Formalized lifecycle state machine for Star agents
//!
//! As Stars join and leave the Grid, formalized lifecycle hooks ensure clean
//! state transitions. This maps naturally to Tokio's graceful shutdown patterns
//! and prevents "orphan jobs" where a Star disappears mid-transfer.
//!
//! # Lifecycle States
//!
//! ```text
//!                    ┌─────────────┐
//!     register() ──> │  Registered │
//!                    └──────┬──────┘
//!                           │ schedule()
//!                    ┌──────▼──────┐
//!                    │  Scheduled  │ ← Actively receiving work
//!                    └──────┬──────┘
//!                           │ drain()
//!                    ┌──────▼──────┐
//!                    │  Draining   │ ← Finishing current work, no new
//!                    └──────┬──────┘
//!                           │ shutdown()
//!                    ┌──────▼──────┐
//!                    │  Shutdown   │
//!                    └─────────────┘
//! ```
//!
//! # Example
//!
//! ```
//! use orbit_star::lifecycle::{StarLifecycle, LifecycleState, LifecycleEvent};
//!
//! let mut lifecycle = StarLifecycle::new("star-1");
//!
//! assert_eq!(lifecycle.state(), LifecycleState::Registered);
//!
//! lifecycle.on_scheduled();
//! assert_eq!(lifecycle.state(), LifecycleState::Scheduled);
//!
//! lifecycle.on_draining();
//! assert_eq!(lifecycle.state(), LifecycleState::Draining);
//! assert!(!lifecycle.accepts_work());
//!
//! lifecycle.on_shutdown();
//! assert_eq!(lifecycle.state(), LifecycleState::Shutdown);
//! ```

use serde::{Deserialize, Serialize};
use std::time::{Instant, SystemTime};

/// Lifecycle states for a Star agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    /// Star is registered but not yet receiving work
    Registered,

    /// Star is actively receiving and processing work
    Scheduled,

    /// Star is finishing current work but not accepting new work
    Draining,

    /// Star has shut down cleanly
    Shutdown,
}

impl LifecycleState {
    /// String representation
    pub fn as_str(&self) -> &str {
        match self {
            LifecycleState::Registered => "registered",
            LifecycleState::Scheduled => "scheduled",
            LifecycleState::Draining => "draining",
            LifecycleState::Shutdown => "shutdown",
        }
    }
}

/// Events emitted during lifecycle transitions
#[derive(Debug, Clone)]
pub struct LifecycleEvent {
    /// Star ID
    pub star_id: String,

    /// Previous state
    pub from: LifecycleState,

    /// New state
    pub to: LifecycleState,

    /// When the transition occurred
    pub timestamp: SystemTime,
}

/// Manages the lifecycle state machine for a single Star agent.
#[derive(Debug)]
pub struct StarLifecycle {
    /// Star identifier
    star_id: String,

    /// Current lifecycle state
    state: LifecycleState,

    /// When the current state was entered
    state_entered_at: Instant,

    /// History of lifecycle events
    events: Vec<LifecycleEvent>,

    /// Number of active tasks (for draining)
    active_tasks: u32,
}

impl StarLifecycle {
    /// Create a new lifecycle manager in the Registered state
    pub fn new(star_id: &str) -> Self {
        Self {
            star_id: star_id.to_string(),
            state: LifecycleState::Registered,
            state_entered_at: Instant::now(),
            events: Vec::new(),
            active_tasks: 0,
        }
    }

    /// Get the current state
    pub fn state(&self) -> LifecycleState {
        self.state
    }

    /// Get the Star ID
    pub fn star_id(&self) -> &str {
        &self.star_id
    }

    /// Check if this Star accepts new work
    pub fn accepts_work(&self) -> bool {
        self.state == LifecycleState::Scheduled
    }

    /// Check if this Star is fully drained (no active tasks)
    pub fn is_drained(&self) -> bool {
        self.state == LifecycleState::Draining && self.active_tasks == 0
    }

    /// Get time spent in the current state
    pub fn time_in_state(&self) -> std::time::Duration {
        self.state_entered_at.elapsed()
    }

    /// Get the number of active tasks
    pub fn active_tasks(&self) -> u32 {
        self.active_tasks
    }

    /// Record a task starting
    pub fn task_started(&mut self) {
        self.active_tasks += 1;
    }

    /// Record a task completing
    pub fn task_completed(&mut self) {
        self.active_tasks = self.active_tasks.saturating_sub(1);
    }

    /// Transition: Registered → Scheduled
    ///
    /// Called when the Star is ready to receive work (pre-warm caches, etc.)
    pub fn on_scheduled(&mut self) -> Option<LifecycleEvent> {
        if self.state != LifecycleState::Registered {
            return None;
        }
        self.transition(LifecycleState::Scheduled)
    }

    /// Transition: Scheduled → Draining
    ///
    /// Called when the Star should finish current work and accept no new.
    pub fn on_draining(&mut self) -> Option<LifecycleEvent> {
        if self.state != LifecycleState::Scheduled {
            return None;
        }
        self.transition(LifecycleState::Draining)
    }

    /// Transition: Draining → Shutdown (or Registered/Scheduled → Shutdown for force)
    ///
    /// Called when the Star is shutting down. In graceful mode, should only
    /// be called after draining is complete.
    pub fn on_shutdown(&mut self) -> Option<LifecycleEvent> {
        if self.state == LifecycleState::Shutdown {
            return None;
        }
        self.transition(LifecycleState::Shutdown)
    }

    /// Get all lifecycle events
    pub fn events(&self) -> &[LifecycleEvent] {
        &self.events
    }

    /// Internal transition helper
    fn transition(&mut self, new_state: LifecycleState) -> Option<LifecycleEvent> {
        let event = LifecycleEvent {
            star_id: self.star_id.clone(),
            from: self.state,
            to: new_state,
            timestamp: SystemTime::now(),
        };

        self.state = new_state;
        self.state_entered_at = Instant::now();
        self.events.push(event.clone());

        Some(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let lc = StarLifecycle::new("star-1");
        assert_eq!(lc.state(), LifecycleState::Registered);
        assert!(!lc.accepts_work());
        assert_eq!(lc.star_id(), "star-1");
    }

    #[test]
    fn test_happy_path_lifecycle() {
        let mut lc = StarLifecycle::new("star-1");

        // Registered → Scheduled
        let event = lc.on_scheduled().unwrap();
        assert_eq!(event.from, LifecycleState::Registered);
        assert_eq!(event.to, LifecycleState::Scheduled);
        assert!(lc.accepts_work());

        // Scheduled → Draining
        let event = lc.on_draining().unwrap();
        assert_eq!(event.from, LifecycleState::Scheduled);
        assert_eq!(event.to, LifecycleState::Draining);
        assert!(!lc.accepts_work());

        // Draining → Shutdown
        let event = lc.on_shutdown().unwrap();
        assert_eq!(event.from, LifecycleState::Draining);
        assert_eq!(event.to, LifecycleState::Shutdown);

        assert_eq!(lc.events().len(), 3);
    }

    #[test]
    fn test_invalid_transitions_return_none() {
        let mut lc = StarLifecycle::new("star-1");

        // Can't drain from Registered
        assert!(lc.on_draining().is_none());

        // Can't schedule twice
        lc.on_scheduled();
        assert!(lc.on_scheduled().is_none());
    }

    #[test]
    fn test_force_shutdown() {
        let mut lc = StarLifecycle::new("star-1");
        lc.on_scheduled();

        // Force shutdown from Scheduled (skip draining)
        let event = lc.on_shutdown().unwrap();
        assert_eq!(event.from, LifecycleState::Scheduled);
        assert_eq!(event.to, LifecycleState::Shutdown);
    }

    #[test]
    fn test_task_tracking() {
        let mut lc = StarLifecycle::new("star-1");

        lc.task_started();
        lc.task_started();
        assert_eq!(lc.active_tasks(), 2);

        lc.task_completed();
        assert_eq!(lc.active_tasks(), 1);

        // Saturating sub
        lc.task_completed();
        lc.task_completed();
        assert_eq!(lc.active_tasks(), 0);
    }

    #[test]
    fn test_is_drained() {
        let mut lc = StarLifecycle::new("star-1");
        lc.on_scheduled();
        lc.task_started();
        lc.on_draining();

        assert!(!lc.is_drained()); // Still has active tasks

        lc.task_completed();
        assert!(lc.is_drained()); // Now drained
    }

    #[test]
    fn test_double_shutdown_returns_none() {
        let mut lc = StarLifecycle::new("star-1");
        lc.on_shutdown();
        assert!(lc.on_shutdown().is_none());
    }

    #[test]
    fn test_lifecycle_state_as_str() {
        assert_eq!(LifecycleState::Registered.as_str(), "registered");
        assert_eq!(LifecycleState::Scheduled.as_str(), "scheduled");
        assert_eq!(LifecycleState::Draining.as_str(), "draining");
        assert_eq!(LifecycleState::Shutdown.as_str(), "shutdown");
    }

    #[test]
    fn test_draining_from_draining_returns_none() {
        let mut lc = StarLifecycle::new("star-1");
        lc.on_scheduled();
        lc.on_draining();
        assert_eq!(lc.state(), LifecycleState::Draining);
        assert!(lc.on_draining().is_none());
    }

    #[test]
    fn test_scheduled_from_draining_returns_none() {
        let mut lc = StarLifecycle::new("star-1");
        lc.on_scheduled();
        lc.on_draining();
        assert_eq!(lc.state(), LifecycleState::Draining);
        assert!(lc.on_scheduled().is_none());
    }

    #[test]
    fn test_scheduled_from_shutdown_returns_none() {
        let mut lc = StarLifecycle::new("star-1");
        lc.on_shutdown();
        assert_eq!(lc.state(), LifecycleState::Shutdown);
        assert!(lc.on_scheduled().is_none());
    }

    #[test]
    fn test_draining_from_shutdown_returns_none() {
        let mut lc = StarLifecycle::new("star-1");
        lc.on_shutdown();
        assert_eq!(lc.state(), LifecycleState::Shutdown);
        assert!(lc.on_draining().is_none());
    }

    #[test]
    fn test_force_shutdown_from_registered() {
        let mut lc = StarLifecycle::new("star-1");
        assert_eq!(lc.state(), LifecycleState::Registered);
        let event = lc.on_shutdown().unwrap();
        assert_eq!(event.from, LifecycleState::Registered);
        assert_eq!(event.to, LifecycleState::Shutdown);
        assert_eq!(lc.state(), LifecycleState::Shutdown);
    }

    #[test]
    fn test_task_started_in_registered_state() {
        let mut lc = StarLifecycle::new("star-1");
        assert_eq!(lc.state(), LifecycleState::Registered);
        lc.task_started();
        assert_eq!(lc.active_tasks(), 1);
        lc.task_started();
        assert_eq!(lc.active_tasks(), 2);
    }

    #[test]
    fn test_is_drained_in_scheduled_state() {
        let mut lc = StarLifecycle::new("star-1");
        lc.on_scheduled();
        assert_eq!(lc.state(), LifecycleState::Scheduled);
        assert_eq!(lc.active_tasks(), 0);
        assert!(!lc.is_drained());
    }

    #[test]
    fn test_is_drained_in_shutdown_state() {
        let mut lc = StarLifecycle::new("star-1");
        lc.on_scheduled();
        lc.on_draining();
        lc.on_shutdown();
        assert_eq!(lc.state(), LifecycleState::Shutdown);
        assert!(!lc.is_drained());
    }

    #[test]
    fn test_event_history_accumulated() {
        let mut lc = StarLifecycle::new("star-1");
        lc.on_scheduled();
        lc.on_draining();
        lc.on_shutdown();

        let events = lc.events();
        assert_eq!(events.len(), 3);

        assert_eq!(events[0].from, LifecycleState::Registered);
        assert_eq!(events[0].to, LifecycleState::Scheduled);

        assert_eq!(events[1].from, LifecycleState::Scheduled);
        assert_eq!(events[1].to, LifecycleState::Draining);

        assert_eq!(events[2].from, LifecycleState::Draining);
        assert_eq!(events[2].to, LifecycleState::Shutdown);
    }

    #[test]
    fn test_time_in_state_increases() {
        let mut lc = StarLifecycle::new("star-1");
        lc.on_scheduled();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(lc.time_in_state() > std::time::Duration::ZERO);
    }

    #[test]
    fn test_multiple_lifecycles_independent() {
        let mut lc1 = StarLifecycle::new("star-a");
        let mut lc2 = StarLifecycle::new("star-b");

        lc1.on_scheduled();
        assert_eq!(lc1.state(), LifecycleState::Scheduled);
        assert_eq!(lc2.state(), LifecycleState::Registered);

        lc2.on_scheduled();
        lc2.on_draining();
        assert_eq!(lc1.state(), LifecycleState::Scheduled);
        assert_eq!(lc2.state(), LifecycleState::Draining);

        assert_eq!(lc1.events().len(), 1);
        assert_eq!(lc2.events().len(), 2);
    }

    #[test]
    fn test_lifecycle_state_serde_roundtrip() {
        let states = [
            LifecycleState::Registered,
            LifecycleState::Scheduled,
            LifecycleState::Draining,
            LifecycleState::Shutdown,
        ];

        for state in &states {
            let json = serde_json::to_string(state).unwrap();
            let deserialized: LifecycleState = serde_json::from_str(&json).unwrap();
            assert_eq!(*state, deserialized);
        }
    }
}
