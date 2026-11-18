//! Authorization: roles, capabilities, and the default-deny policy of
//! docs/10-security.md §4. Pure types and functions; the enforcement
//! middleware lives in fantatt-api.

use serde::{Deserialize, Serialize};

use crate::ids::Identifier;

/// A role in the capability model of docs/10-security.md §4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Role {
    Superadmin,
    LeagueAdmin,
    Collaborator,
    Manager,
    Member,
}

impl Role {
    /// Spelling stored in `fantatt.memberships.role`.
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Superadmin => "SUPERADMIN",
            Self::LeagueAdmin => "LEAGUE_ADMIN",
            Self::Collaborator => "COLLABORATOR",
            Self::Manager => "MANAGER",
            Self::Member => "MEMBER",
        }
    }

    /// Parses a stored spelling back into a role.
    pub fn from_db_str(text: &str) -> Option<Self> {
        match text {
            "SUPERADMIN" => Some(Self::Superadmin),
            "LEAGUE_ADMIN" => Some(Self::LeagueAdmin),
            "COLLABORATOR" => Some(Self::Collaborator),
            "MANAGER" => Some(Self::Manager),
            "MEMBER" => Some(Self::Member),
            _ => None,
        }
    }
}

/// A permission every route declares before it may serve a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Capability {
    Public,
    Session,
    LeagueRead,
    LeagueWrite,
    RostersWrite,
    SquadOwn,
    UsersRead,
    UsersWriteRoles,
    PlatformAdmin,
}

impl Capability {
    /// Wire name used in problem details and route tables.
    pub fn name(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Session => "session",
            Self::LeagueRead => "league:read",
            Self::LeagueWrite => "league:write",
            Self::RostersWrite => "rosters:write",
            Self::SquadOwn => "squad:own",
            Self::UsersRead => "users:read",
            Self::UsersWriteRoles => "users:write:roles",
            Self::PlatformAdmin => "platform:admin",
        }
    }

    /// Parses a wire name back into a capability.
    pub fn from_name(text: &str) -> Option<Self> {
        match text {
            "public" => Some(Self::Public),
            "session" => Some(Self::Session),
            "league:read" => Some(Self::LeagueRead),
            "league:write" => Some(Self::LeagueWrite),
            "rosters:write" => Some(Self::RostersWrite),
            "squad:own" => Some(Self::SquadOwn),
            "users:read" => Some(Self::UsersRead),
            "users:write:roles" => Some(Self::UsersWriteRoles),
            "platform:admin" => Some(Self::PlatformAdmin),
            _ => None,
        }
    }
}

/// The object a capability is exercised against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectScope {
    Platform,
    League(Identifier),
    Account(Identifier),
}

/// A role held in one league.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeagueRole {
    league_id: Identifier,
    role: Role,
}

impl LeagueRole {
    pub fn new(league_id: Identifier, role: Role) -> Self {
        Self { league_id, role }
    }
}

/// The resolved session principal: identity, platform grant, league roles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    user_id: Identifier,
    superadmin: bool,
    league_roles: Vec<LeagueRole>,
}

impl Principal {
    pub fn new(user_id: Identifier, superadmin: bool, league_roles: Vec<LeagueRole>) -> Self {
        Self {
            user_id,
            superadmin,
            league_roles,
        }
    }

    pub fn user_id(&self) -> &Identifier {
        &self.user_id
    }

    pub fn is_superadmin(&self) -> bool {
        self.superadmin
    }

    /// The role held in one league, if any.
    pub fn role_in(&self, league_id: &Identifier) -> Option<Role> {
        self.league_roles
            .iter()
            .find(|entry| &entry.league_id == league_id)
            .map(|entry| entry.role)
    }
}

/// Outcome of a policy check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

const LEAGUE_READ_ROLES: [Role; 4] = [
    Role::LeagueAdmin,
    Role::Collaborator,
    Role::Manager,
    Role::Member,
];
const LEAGUE_WRITE_ROLES: [Role; 1] = [Role::LeagueAdmin];
const ROSTERS_WRITE_ROLES: [Role; 2] = [Role::LeagueAdmin, Role::Collaborator];
const SQUAD_OWN_ROLES: [Role; 2] = [Role::LeagueAdmin, Role::Manager];

/// Default-deny policy check: an undeclared capability never reaches a
/// handler, and anonymous callers only ever access `Public` routes.
pub fn authorize(
    capability: Capability,
    scope: &ObjectScope,
    principal: Option<&Principal>,
) -> Decision {
    match capability {
        Capability::Public => Decision::Allow,
        Capability::Session => authorize_session(scope, principal),
        Capability::LeagueRead => authorize_league(scope, principal, &LEAGUE_READ_ROLES),
        Capability::LeagueWrite => authorize_league(scope, principal, &LEAGUE_WRITE_ROLES),
        Capability::RostersWrite => authorize_league(scope, principal, &ROSTERS_WRITE_ROLES),
        Capability::SquadOwn => authorize_league(scope, principal, &SQUAD_OWN_ROLES),
        Capability::UsersRead | Capability::UsersWriteRoles | Capability::PlatformAdmin => {
            authorize_platform(scope, principal)
        }
    }
}

fn authorize_session(scope: &ObjectScope, principal: Option<&Principal>) -> Decision {
    let Some(principal) = principal else {
        return Decision::Deny;
    };
    match scope {
        ObjectScope::Platform => Decision::Allow,
        ObjectScope::League(league_id) => {
            if principal.is_superadmin() || principal.role_in(league_id).is_some() {
                Decision::Allow
            } else {
                Decision::Deny
            }
        }
        ObjectScope::Account(account_id) => {
            if principal.user_id() == account_id || principal.is_superadmin() {
                Decision::Allow
            } else {
                Decision::Deny
            }
        }
    }
}

fn authorize_league(
    scope: &ObjectScope,
    principal: Option<&Principal>,
    roles: &[Role],
) -> Decision {
    let Some(principal) = principal else {
        return Decision::Deny;
    };
    if principal.is_superadmin() {
        return Decision::Allow;
    }
    let ObjectScope::League(league_id) = scope else {
        return Decision::Deny;
    };
    match principal.role_in(league_id) {
        Some(role) if roles.contains(&role) => Decision::Allow,
        _ => Decision::Deny,
    }
}

fn authorize_platform(scope: &ObjectScope, principal: Option<&Principal>) -> Decision {
    let ObjectScope::Platform = scope else {
        return Decision::Deny;
    };
    match principal {
        Some(principal) if principal.is_superadmin() => Decision::Allow,
        _ => Decision::Deny,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::DomainPrefix;
    use proptest::prelude::*;

    fn id(prefix: DomainPrefix, last: char) -> Identifier {
        let suffix = format!("01ARZ3XK7M0C4V6B8N2QWE9RT{last}");
        Identifier::new(prefix, &suffix).expect("valid id")
    }

    fn league() -> Identifier {
        id(DomainPrefix::League, 'A')
    }

    fn user() -> Identifier {
        id(DomainPrefix::User, 'B')
    }

    fn other_league() -> Identifier {
        id(DomainPrefix::League, 'Z')
    }

    fn principal(role: Option<Role>) -> Principal {
        let league_roles = role
            .map(|role| vec![LeagueRole::new(league(), role)])
            .unwrap_or_default();
        Principal::new(user(), false, league_roles)
    }

    fn scoped() -> ObjectScope {
        ObjectScope::League(league())
    }

    fn everything() -> Principal {
        Principal::new(user(), true, Vec::new())
    }

    const ROLES: [Role; 5] = [
        Role::Superadmin,
        Role::LeagueAdmin,
        Role::Collaborator,
        Role::Manager,
        Role::Member,
    ];

    const CAPABILITIES: [Capability; 9] = [
        Capability::Public,
        Capability::Session,
        Capability::LeagueRead,
        Capability::LeagueWrite,
        Capability::RostersWrite,
        Capability::SquadOwn,
        Capability::UsersRead,
        Capability::UsersWriteRoles,
        Capability::PlatformAdmin,
    ];

    const PLATFORM_CAPABILITIES: [Capability; 3] = [
        Capability::UsersRead,
        Capability::UsersWriteRoles,
        Capability::PlatformAdmin,
    ];

    const SCOPED_CAPABILITIES: [Capability; 5] = [
        Capability::Session,
        Capability::LeagueRead,
        Capability::LeagueWrite,
        Capability::RostersWrite,
        Capability::SquadOwn,
    ];

    #[test]
    fn db_and_wire_names_round_trip() {
        for role in ROLES {
            assert_eq!(Role::from_db_str(role.as_db_str()), Some(role));
        }
        for capability in CAPABILITIES {
            assert_eq!(Capability::from_name(capability.name()), Some(capability));
        }
    }

    #[test]
    fn public_capability_allows_everyone() {
        let scopes = [
            ObjectScope::Platform,
            scoped(),
            ObjectScope::Account(user()),
        ];
        let callers = [None, Some(principal(None)), Some(everything())];
        for scope in &scopes {
            for caller in &callers {
                let decision = authorize(Capability::Public, scope, caller.as_ref());
                assert_eq!(decision, Decision::Allow);
            }
        }
    }

    #[test]
    fn anonymous_callers_are_denied_every_non_public_capability() {
        for capability in CAPABILITIES {
            if capability == Capability::Public {
                continue;
            }
            let scope = scoped();
            assert_eq!(authorize(capability, &scope, None), Decision::Deny);
        }
    }

    #[test]
    fn league_read_allows_every_league_role() {
        for role in LEAGUE_READ_ROLES {
            let scope = scoped();
            assert_eq!(
                authorize(Capability::LeagueRead, &scope, Some(&principal(Some(role)))),
                Decision::Allow
            );
        }
    }

    #[test]
    fn league_write_allows_only_the_league_admin() {
        for role in LEAGUE_READ_ROLES {
            let scope = scoped();
            let expected = if role == Role::LeagueAdmin {
                Decision::Allow
            } else {
                Decision::Deny
            };
            assert_eq!(
                authorize(
                    Capability::LeagueWrite,
                    &scope,
                    Some(&principal(Some(role)))
                ),
                expected
            );
        }
    }

    #[test]
    fn rosters_write_allows_admins_and_collaborators() {
        for role in LEAGUE_READ_ROLES {
            let scope = scoped();
            let expected = if matches!(role, Role::LeagueAdmin | Role::Collaborator) {
                Decision::Allow
            } else {
                Decision::Deny
            };
            assert_eq!(
                authorize(
                    Capability::RostersWrite,
                    &scope,
                    Some(&principal(Some(role)))
                ),
                expected
            );
        }
    }

    #[test]
    fn squad_own_allows_managers_and_league_admins() {
        for role in LEAGUE_READ_ROLES {
            let scope = scoped();
            let expected = if matches!(role, Role::LeagueAdmin | Role::Manager) {
                Decision::Allow
            } else {
                Decision::Deny
            };
            assert_eq!(
                authorize(Capability::SquadOwn, &scope, Some(&principal(Some(role)))),
                expected
            );
        }
    }

    #[test]
    fn session_capability_follows_the_object_scope() {
        let member = principal(Some(Role::Member));
        let stranger = principal(None);
        let own = ObjectScope::Account(user());
        let foreign = ObjectScope::Account(id(DomainPrefix::User, 'C'));
        let cases = [
            (ObjectScope::Platform, member.clone(), Decision::Allow),
            (scoped(), member.clone(), Decision::Allow),
            (own, member.clone(), Decision::Allow),
            (foreign, member, Decision::Deny),
            (scoped(), stranger, Decision::Deny),
        ];
        for (scope, principal, expected) in cases {
            let decision = authorize(Capability::Session, &scope, Some(&principal));
            assert_eq!(decision, expected);
        }
    }

    #[test]
    fn platform_capabilities_require_the_superadmin_flag_and_platform_scope() {
        let member = principal(Some(Role::Member));
        let cases = [
            (ObjectScope::Platform, everything(), Decision::Allow),
            (ObjectScope::Platform, member, Decision::Deny),
            (scoped(), everything(), Decision::Deny),
        ];
        for capability in PLATFORM_CAPABILITIES {
            for (scope, principal, expected) in &cases {
                let decision = authorize(capability, scope, Some(principal));
                assert_eq!(decision, *expected);
            }
        }
    }

    #[test]
    fn a_role_held_elsewhere_does_not_open_another_league() {
        let scope = ObjectScope::League(other_league());
        let member = principal(Some(Role::Manager));
        assert_eq!(
            authorize(Capability::LeagueRead, &scope, Some(&member)),
            Decision::Deny
        );
    }

    #[test]
    fn superadmin_bypasses_membership_restrictions() {
        let scopes = [
            ObjectScope::Platform,
            scoped(),
            ObjectScope::Account(id(DomainPrefix::User, 'C')),
        ];
        for scope in &scopes {
            for capability in SCOPED_CAPABILITIES {
                let decision = authorize(capability, scope, Some(&everything()));
                assert_eq!(decision, Decision::Allow);
            }
        }
    }

    proptest! {
        #[test]
        fn unknown_role_spellings_are_rejected(text in "\\PC{0,24}") {
            if let Some(role) = Role::from_db_str(&text) {
                prop_assert_eq!(role.as_db_str(), text);
            }
        }

        #[test]
        fn unknown_capability_names_are_rejected(text in "\\PC{0,24}") {
            if let Some(capability) = Capability::from_name(&text) {
                prop_assert_eq!(capability.name(), text);
            }
        }

        #[test]
        fn the_policy_always_answers_for_every_role(
            role in prop_oneof![Just(Role::Superadmin), Just(Role::LeagueAdmin), Just(Role::Collaborator), Just(Role::Manager), Just(Role::Member)],
            capability in prop_oneof![Just(Capability::Public), Just(Capability::Session), Just(Capability::LeagueRead), Just(Capability::LeagueWrite), Just(Capability::RostersWrite), Just(Capability::SquadOwn), Just(Capability::UsersRead), Just(Capability::UsersWriteRoles), Just(Capability::PlatformAdmin)],
            ulid in "[0-9A-HJKMNP-TV-Z]{26}",
        ) {
            let scope = ObjectScope::League(Identifier::new(DomainPrefix::League, &ulid).expect("valid"));
            let decision = authorize(capability, &scope, Some(&principal(Some(role))));
            prop_assert!(matches!(decision, Decision::Allow | Decision::Deny));
        }
    }
}
