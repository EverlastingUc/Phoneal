//! Deny-by-default capability policy primitives.

pub mod ipc;
//!
//! These types model grants for an embedding host. They do not perform host I/O,
//! create OS capabilities, or isolate the VM. Enforcement requires the future
//! parent-process broker and restricted worker described in the security architecture.

use std::{
    collections::BTreeSet,
    error::Error,
    fmt,
};

/// Host operations that may eventually be exposed through a restricted broker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    ConsoleRead,
    FileRead,
    FileWrite,
    NetworkConnect,
    Clock,
    Random,
}

impl Capability {
    pub const ALL: [Self; 6] = [
        Self::ConsoleRead,
        Self::FileRead,
        Self::FileWrite,
        Self::NetworkConnect,
        Self::Clock,
        Self::Random,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ConsoleRead => "console-read",
            Self::FileRead => "file-read",
            Self::FileWrite => "file-write",
            Self::NetworkConnect => "network-connect",
            Self::Clock => "clock",
            Self::Random => "random",
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A policy contains only explicitly granted capabilities. Its default is deny-all.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapabilityPolicy {
    grants: BTreeSet<Capability>,
}

impl CapabilityPolicy {
    /// Creates a policy that denies every host capability.
    pub fn deny_all() -> Self {
        Self::default()
    }

    /// Creates a policy containing exactly the supplied grants.
    pub fn with_grants(grants: impl IntoIterator<Item = Capability>) -> Self {
        Self {
            grants: grants.into_iter().collect(),
        }
    }

    /// Adds one explicit grant.
    pub fn grant(&mut self, capability: Capability) {
        self.grants.insert(capability);
    }

    /// Revokes one grant. Revoking an absent grant is harmless.
    pub fn revoke(&mut self, capability: Capability) {
        self.grants.remove(&capability);
    }

    /// Returns whether this exact capability is granted.
    pub fn allows(&self, capability: Capability) -> bool {
        self.grants.contains(&capability)
    }

    /// Checks a capability request without performing the requested operation.
    pub fn authorize(&self, capability: Capability) -> Result<(), CapabilityDenied> {
        if self.allows(capability) {
            Ok(())
        } else {
            Err(CapabilityDenied { capability })
        }
    }

    /// Iterates over explicitly granted capabilities in stable order.
    pub fn grants(&self) -> impl Iterator<Item = Capability> + '_ {
        self.grants.iter().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.grants.is_empty()
    }
}

/// Returned when a policy does not contain the requested capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityDenied {
    pub capability: Capability,
}

impl fmt::Display for CapabilityDenied {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Capability denied: {}", self.capability)
    }
}

impl Error for CapabilityDenied {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_denies_every_capability() {
        let policy = CapabilityPolicy::default();
        assert!(policy.is_empty());
        for capability in Capability::ALL {
            assert!(!policy.allows(capability));
            assert_eq!(
                policy.authorize(capability),
                Err(CapabilityDenied { capability })
            );
        }
    }

    #[test]
    fn grants_are_explicit_and_do_not_imply_other_permissions() {
        let policy = CapabilityPolicy::with_grants([Capability::FileRead]);
        assert!(policy.allows(Capability::FileRead));
        assert!(!policy.allows(Capability::FileWrite));
        assert!(!policy.allows(Capability::NetworkConnect));
        assert_eq!(policy.authorize(Capability::FileRead), Ok(()));
    }

    #[test]
    fn grants_can_be_added_and_revoked() {
        let mut policy = CapabilityPolicy::deny_all();
        policy.grant(Capability::Clock);
        assert!(policy.allows(Capability::Clock));
        policy.revoke(Capability::Clock);
        policy.revoke(Capability::Random);
        assert!(!policy.allows(Capability::Clock));
        assert!(policy.is_empty());
    }

    #[test]
    fn capability_listing_is_stable_and_exact() {
        let policy = CapabilityPolicy::with_grants([
            Capability::NetworkConnect,
            Capability::FileRead,
            Capability::FileRead,
        ]);
        assert_eq!(
            policy.grants().collect::<Vec<_>>(),
            vec![Capability::FileRead, Capability::NetworkConnect]
        );
    }
}
