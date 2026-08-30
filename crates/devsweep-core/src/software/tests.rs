use super::*;

fn observation(identity: SoftwareIdentity, flags: EligibilityFlags) -> SoftwareObservation {
    SoftwareObservation {
        scope: match &identity {
            SoftwareIdentity::Arp { hive, .. } => {
                if *hive == RegistryHive::CurrentUser {
                    SoftwareScope::CurrentUser
                } else {
                    SoftwareScope::Machine
                }
            }
            SoftwareIdentity::Msi { context, .. } => {
                if *context == MsiContext::Machine {
                    SoftwareScope::Machine
                } else {
                    SoftwareScope::CurrentUser
                }
            }
            SoftwareIdentity::Msix { .. } => SoftwareScope::CurrentUser,
        },
        identity,
        display_name: Some("Fixture".to_string()),
        publisher: Some("Fixture Publisher".to_string()),
        version: Some("1.0".to_string()),
        provenance: vec![SoftwareSourceId::MsixCurrentUser],
        size: SoftwareSizeEvidence::Unknown {
            reason_code: "fixture".to_string(),
        },
        flags,
    }
}

fn msix(name: &str, flags: EligibilityFlags) -> SoftwareObservation {
    observation(
        SoftwareIdentity::Msix {
            package_full_name: name.to_string(),
        },
        flags,
    )
}

#[test]
fn inventory_order_and_fingerprint_are_deterministic() {
    let source = SoftwareSourceEvidence::available(SoftwareSourceId::MsixCurrentUser);
    let left = assemble_inventory(
        10,
        vec![source.clone()],
        vec![
            msix("B_1.0.0.0_x64__publisher", EligibilityFlags::default()),
            msix("A_1.0.0.0_x64__publisher", EligibilityFlags::default()),
        ],
    )
    .unwrap();
    let right = assemble_inventory(
        10,
        vec![source],
        vec![
            msix("A_1.0.0.0_x64__publisher", EligibilityFlags::default()),
            msix("B_1.0.0.0_x64__publisher", EligibilityFlags::default()),
        ],
    )
    .unwrap();
    assert_eq!(left, right);
    assert!(
        left.entries
            .iter()
            .all(|entry| entry.eligibility.is_selectable())
    );
}

#[test]
fn refusal_matrix_uses_the_exact_first_matching_priority() {
    use SoftwareEligibilityReason as Reason;

    let cases = [
        (
            EligibilityFlags {
                protected: true,
                source_incomplete: true,
                ..EligibilityFlags::default()
            },
            Reason::ProtectedProduct,
        ),
        (
            EligibilityFlags {
                source_incomplete: true,
                conflicting_identity: true,
                ..EligibilityFlags::default()
            },
            Reason::SourceIncomplete,
        ),
        (
            EligibilityFlags {
                conflicting_identity: true,
                no_remove: true,
                ..EligibilityFlags::default()
            },
            Reason::ConflictingIdentity,
        ),
        (
            EligibilityFlags {
                no_remove: true,
                hidden: true,
                ..EligibilityFlags::default()
            },
            Reason::NoRemove,
        ),
        (
            EligibilityFlags {
                hidden: true,
                system_or_update: true,
                ..EligibilityFlags::default()
            },
            Reason::HiddenEntry,
        ),
        (
            EligibilityFlags {
                system_or_update: true,
                dependency: true,
                ..EligibilityFlags::default()
            },
            Reason::SystemOrUpdate,
        ),
        (
            EligibilityFlags {
                dependency: true,
                stub: true,
                ..EligibilityFlags::default()
            },
            Reason::DependencyPackage,
        ),
        (
            EligibilityFlags {
                stub: true,
                unhealthy: true,
                ..EligibilityFlags::default()
            },
            Reason::StubPackage,
        ),
        (
            EligibilityFlags {
                unhealthy: true,
                unsupported: true,
                ..EligibilityFlags::default()
            },
            Reason::UnhealthyPackage,
        ),
        (
            EligibilityFlags {
                unsupported: true,
                ..EligibilityFlags::default()
            },
            Reason::UnsupportedSource,
        ),
        (EligibilityFlags::default(), Reason::EligibleCurrentUserMsix),
    ];
    for (index, (flags, expected)) in cases.into_iter().enumerate() {
        let entry = msix(&format!("Fixture{index}_1.0.0.0_x64__publisher"), flags);
        assert_eq!(
            ordered_eligibility(&entry.identity, entry.flags).reason,
            expected
        );
    }

    let msi = observation(
        SoftwareIdentity::Msi {
            product_code: "{00000000-0000-0000-0000-000000000001}".to_string(),
            context: MsiContext::Machine,
        },
        EligibilityFlags::default(),
    );
    assert_eq!(
        ordered_eligibility(&msi.identity, msi.flags).reason,
        Reason::MsiExecutionNotSupportedV1
    );
    let arp = observation(
        SoftwareIdentity::Arp {
            hive: RegistryHive::CurrentUser,
            view: RegistryView::Registry64,
            subkey: "fixture".to_string(),
        },
        EligibilityFlags::default(),
    );
    assert_eq!(
        ordered_eligibility(&arp.identity, arp.flags).reason,
        Reason::RegistryOnlyManual
    );
}

#[test]
fn denied_or_partial_sources_do_not_erase_complete_siblings() {
    let inventory = assemble_inventory(
        1,
        vec![
            SoftwareSourceEvidence::unavailable(
                SoftwareSourceId::Arp {
                    hive: RegistryHive::LocalMachine,
                    view: RegistryView::Registry64,
                },
                SoftwareSourceState::Permission,
                "registry_access_denied",
            ),
            SoftwareSourceEvidence::available(SoftwareSourceId::MsixCurrentUser),
        ],
        vec![msix(
            "Complete_1.0.0.0_x64__publisher",
            EligibilityFlags::default(),
        )],
    )
    .unwrap();
    assert_eq!(inventory.sources.len(), 2);
    assert_eq!(inventory.entries.len(), 1);
    assert!(inventory.entries[0].eligibility.is_selectable());
}

#[test]
fn exact_identity_conflicts_do_not_fuzzy_merge_duplicate_display_names() {
    let first = msix("A_1.0.0.0_x64__publisher", EligibilityFlags::default());
    let mut second = msix("B_1.0.0.0_x64__publisher", EligibilityFlags::default());
    second.display_name = first.display_name.clone();
    let inventory = assemble_inventory(
        1,
        vec![SoftwareSourceEvidence::available(
            SoftwareSourceId::MsixCurrentUser,
        )],
        vec![first, second],
    )
    .unwrap();
    assert_eq!(inventory.entries.len(), 2);

    let mut conflicting = msix("C_1.0.0.0_x64__publisher", EligibilityFlags::default());
    let mut duplicate = conflicting.clone();
    duplicate.version = Some("2.0".to_string());
    conflicting.display_name = Some("one".to_string());
    duplicate.display_name = Some("two".to_string());
    let inventory = assemble_inventory(1, vec![], vec![conflicting, duplicate]).unwrap();
    assert_eq!(inventory.entries.len(), 1);
    assert_eq!(
        inventory.entries[0].eligibility.reason,
        SoftwareEligibilityReason::ConflictingIdentity
    );
}

#[test]
fn exact_duplicate_merging_is_order_independent_and_preserves_refusals() {
    let identity = SoftwareIdentity::Msix {
        package_full_name: "Duplicate_1.0.0.0_x64__publisher".to_string(),
    };
    let base = observation(identity, EligibilityFlags::default());
    let mut no_remove = base.clone();
    no_remove.flags.no_remove = true;

    let left = assemble_inventory(1, vec![], vec![base.clone(), no_remove.clone()]).unwrap();
    let right = assemble_inventory(1, vec![], vec![no_remove, base.clone()]).unwrap();
    assert_eq!(left, right);
    assert_eq!(
        left.entries[0].eligibility.reason,
        SoftwareEligibilityReason::NoRemove
    );

    let mut conflicting = base.clone();
    conflicting.display_name = Some("Different display observation".to_string());
    conflicting.version = Some("2.0".to_string());
    let left = assemble_inventory(1, vec![], vec![base.clone(), conflicting.clone()]).unwrap();
    let right = assemble_inventory(1, vec![], vec![conflicting, base]).unwrap();
    assert_eq!(left, right);
    assert_eq!(
        left.entries[0].eligibility.reason,
        SoftwareEligibilityReason::ConflictingIdentity
    );
    assert_eq!(left.entries[0].display_name, None);
    assert_eq!(left.entries[0].version, None);
}

#[test]
fn reported_kib_uses_checked_integer_arithmetic() {
    assert!(matches!(
        reported_size(Some(2), SoftwareSizeSourceCode::ArpEstimatedSizeKib, 1),
        SoftwareSizeEvidence::Available {
            value_bytes: 2048,
            ..
        }
    ));
    assert!(matches!(
        reported_size(
            Some(u64::MAX),
            SoftwareSizeSourceCode::MsiEstimatedSizeKib,
            1
        ),
        SoftwareSizeEvidence::Unknown { .. }
    ));
}
