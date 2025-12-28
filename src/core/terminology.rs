/*!
 * Terminology System - User-Facing Display Layer
 *
 * This module provides a translation layer between internal architectural
 * names and external user-facing terminology. This abstraction improves
 * usability without sacrificing debuggability.
 *
 * Version: 0.7.0
 * Phase: 3 - Terminology Abstraction
 */

use std::fmt;

/// Internal architectural components with user-friendly mappings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Component {
    /// Job execution engine (internal: Magnetar)
    Magnetar,
    /// Transfer manifest system (internal: Starmap)
    Starmap,
    /// Small file optimization system (internal: Neutrino)
    Neutrino,
    /// Global deduplication index (internal: Universe)
    Universe,
    /// Grid protocol (internal: Star Protocol)
    StarProtocol,
}

impl Component {
    /// Returns the user-friendly name for standard output (CLI/Dashboard)
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Magnetar => "Job Engine",
            Self::Starmap => "Transfer Manifest",
            Self::Neutrino => "Small File Optimization",
            Self::Universe => "Global Index",
            Self::StarProtocol => "Grid Protocol",
        }
    }

    /// Returns the architectural name for debug logs and internal use
    pub fn debug_name(&self) -> &'static str {
        match self {
            Self::Magnetar => "Magnetar",
            Self::Starmap => "Starmap",
            Self::Neutrino => "Neutrino",
            Self::Universe => "Universe",
            Self::StarProtocol => "Star Protocol",
        }
    }

    /// Returns the internal module path for tracing targets
    pub fn module_path(&self) -> &'static str {
        match self {
            Self::Magnetar => "magnetar::core",
            Self::Starmap => "starmap::graph",
            Self::Neutrino => "neutrino::fast_lane",
            Self::Universe => "universe::dedup",
            Self::StarProtocol => "orbit_star::proto",
        }
    }
}

impl fmt::Display for Component {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Status messages with user-facing translations
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub component: Component,
    pub internal_status: String,
    pub user_message: String,
}

impl StatusMessage {
    /// Create a new status message with automatic translation
    pub fn new(component: Component, internal_status: &str) -> Self {
        let user_message = translate_status(component, internal_status);
        Self {
            component,
            internal_status: internal_status.to_string(),
            user_message,
        }
    }

    /// Get the user-facing display version
    pub fn display(&self) -> String {
        format!("{}: {}", self.component.display_name(), self.user_message)
    }

    /// Get the debug version with internal details
    pub fn debug(&self) -> String {
        format!(
            "{} ({}): {}",
            self.component.display_name(),
            self.component.debug_name(),
            self.internal_status
        )
    }
}

/// Translate internal status codes to user-friendly messages
fn translate_status(component: Component, internal_status: &str) -> String {
    match (component, internal_status) {
        // Magnetar (Job Engine) translations
        (Component::Magnetar, "persisting") => "Saving State...".to_string(),
        (Component::Magnetar, "executing") => "Executing Transfer...".to_string(),
        (Component::Magnetar, "queued") => "Queued for Execution...".to_string(),
        (Component::Magnetar, "circuit_open") => "Paused (Too Many Errors)".to_string(),

        // Starmap (Transfer Manifest) translations
        (Component::Starmap, "calculating") => "Calculating Dependencies...".to_string(),
        (Component::Starmap, "generating") => "Generating Manifest...".to_string(),
        (Component::Starmap, "verifying") => "Verifying Transfer Plan...".to_string(),

        // Neutrino (Small File Optimization) translations
        (Component::Neutrino, "active") => "Optimizing Small Files...".to_string(),
        (Component::Neutrino, "batching") => "Batching Small Files...".to_string(),
        (Component::Neutrino, "bypassed") => "Using Standard Transfer".to_string(),

        // Universe (Global Index) translations
        (Component::Universe, "indexing") => "Indexing Content...".to_string(),
        (Component::Universe, "deduplicating") => "Removing Duplicates...".to_string(),

        // Star Protocol (Grid Protocol) translations
        (Component::StarProtocol, "connecting") => "Connecting to Grid...".to_string(),
        (Component::StarProtocol, "negotiating") => "Negotiating Protocol...".to_string(),

        // Default: pass through with cleanup
        _ => internal_status.replace("_", " ").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_display_names() {
        assert_eq!(Component::Magnetar.display_name(), "Job Engine");
        assert_eq!(Component::Starmap.display_name(), "Transfer Manifest");
        assert_eq!(
            Component::Neutrino.display_name(),
            "Small File Optimization"
        );
        assert_eq!(Component::Universe.display_name(), "Global Index");
        assert_eq!(Component::StarProtocol.display_name(), "Grid Protocol");
    }

    #[test]
    fn test_component_debug_names() {
        assert_eq!(Component::Magnetar.debug_name(), "Magnetar");
        assert_eq!(Component::Starmap.debug_name(), "Starmap");
        assert_eq!(Component::Neutrino.debug_name(), "Neutrino");
    }

    #[test]
    fn test_component_module_paths() {
        assert_eq!(Component::Magnetar.module_path(), "magnetar::core");
        assert_eq!(Component::Starmap.module_path(), "starmap::graph");
    }

    #[test]
    fn test_status_message_translation() {
        let msg = StatusMessage::new(Component::Magnetar, "persisting");
        assert_eq!(msg.display(), "Job Engine: Saving State...");
        assert!(msg.debug().contains("Magnetar"));
    }

    #[test]
    fn test_status_translation() {
        assert_eq!(
            translate_status(Component::Magnetar, "persisting"),
            "Saving State..."
        );
        assert_eq!(
            translate_status(Component::Neutrino, "active"),
            "Optimizing Small Files..."
        );
    }

    #[test]
    fn test_component_display_trait() {
        assert_eq!(format!("{}", Component::Magnetar), "Job Engine");
        assert_eq!(
            format!("{}", Component::Neutrino),
            "Small File Optimization"
        );
    }

    #[test]
    fn test_unknown_status_cleanup() {
        let result = translate_status(Component::Magnetar, "some_unknown_status");
        assert_eq!(result, "some unknown status");
    }
}
