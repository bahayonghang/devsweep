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
        icon: None,
        installed_on: None,
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
fn source_worker_panic_fallbacks_keep_sibling_sources_intact() {
    let arp = super::failed_arp_inventory();
    assert!(arp.observations.is_empty());
    assert_eq!(arp.evidence.len(), 4);
    assert!(arp.evidence.iter().all(|evidence| {
        evidence.state == SoftwareSourceState::Partial
            && evidence.reason_code.as_deref() == Some("source_worker_panicked")
    }));

    let msi = super::failed_msi_inventory();
    assert!(msi.observations.is_empty());
    assert_eq!(msi.evidence.len(), 3);
    assert!(msi.evidence.iter().all(|evidence| {
        evidence.state == SoftwareSourceState::Partial
            && evidence.reason_code.as_deref() == Some("source_worker_panicked")
    }));
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

#[test]
fn legacy_inventory_keeps_its_fixed_fingerprint_and_omits_absent_evidence() {
    let raw = include_str!("../../tests/fixtures/software/inventory-legacy.json");
    let inventory: SoftwareInventoryV1 = serde_json::from_str(raw).unwrap();
    assert_eq!(
        inventory.fingerprint,
        "sha256:78020d9bcae6440a1857e3e9de9b8c83d52c9663ed2164a03821b6afbbd30457"
    );
    assert_eq!(
        inventory_fingerprint(&inventory).unwrap(),
        inventory.fingerprint
    );
    assert_eq!(inventory.entries[0].icon, None);
    assert_eq!(inventory.entries[0].installed_on, None);
    assert_eq!(
        serde_json::to_value(&inventory).unwrap(),
        serde_json::from_str::<serde_json::Value>(raw).unwrap()
    );
}

#[test]
fn optional_evidence_round_trips_without_command_fields_or_identity_changes() {
    let raw = include_str!("../../tests/fixtures/software/inventory-evidence.json");
    let inventory: SoftwareInventoryV1 = serde_json::from_str(raw).unwrap();
    assert_eq!(
        inventory_fingerprint(&inventory).unwrap(),
        inventory.fingerprint
    );
    assert_eq!(inventory.entries[0].icon.as_ref().unwrap().index, -101);
    assert_eq!(
        validated_installed_on(inventory.entries[0].installed_on.as_deref()),
        Some("2024-02-29")
    );
    let mut absent = inventory.clone();
    absent.entries[0].icon = None;
    absent.entries[0].installed_on = None;
    assert_eq!(absent.entries[0].id, inventory.entries[0].id);
    assert_eq!(
        absent.entries[0].eligibility,
        inventory.entries[0].eligibility
    );
    assert_ne!(
        inventory_fingerprint(&absent).unwrap(),
        inventory.fingerprint
    );
    let json = serde_json::to_value(&inventory).unwrap();
    for forbidden in [
        ["Uninstall", "String"].concat(),
        ["Quiet", "Uninstall", "String"].concat(),
        "program".to_string(),
        "argv".to_string(),
    ] {
        assert!(!serde_json::to_string(&json).unwrap().contains(&forbidden));
    }
    assert_eq!(
        serde_json::from_value::<SoftwareInventoryV1>(json).unwrap(),
        inventory
    );
}

#[test]
fn conflicting_display_evidence_becomes_unknown_without_eligibility_refusal() {
    let mut first = msix(
        "Evidence_1.0.0.0_x64__publisher",
        EligibilityFlags::default(),
    );
    first.icon = Some(IconEvidenceV1 {
        path: r"C:\Apps\first.ico".to_string(),
        index: 0,
    });
    first.installed_on = Some("2026-11-01".to_string());
    let mut second = first.clone();
    second.icon.as_mut().unwrap().index = 1;
    second.installed_on = Some("2026-11-02".to_string());
    let left = assemble_inventory(1, vec![], vec![first.clone(), second.clone()]).unwrap();
    let right = assemble_inventory(1, vec![], vec![second, first]).unwrap();
    assert_eq!(left, right);
    assert_eq!(left.entries[0].icon, None);
    assert_eq!(left.entries[0].installed_on, None);
    assert!(left.entries[0].eligibility.is_selectable());
}
