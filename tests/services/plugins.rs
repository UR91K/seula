//! `PluginsService` tests. Ported from `tests/grpc/plugins.rs` (ADR-0046).

use super::{create_test_project_in_db, test_env};
use seula::database::plugins::InstallState;
use seula::database::ProjectScope;
use seula::error::DatabaseError;
use seula::scan::plugins::ScanMode;

/// Seeded, so the loops over rows below really run: `create_test_project_in_db` brings
/// a plugin with it.
#[tokio::test]
async fn plugin_vendors_have_consistent_counts() {
    let env = test_env();
    create_test_project_in_db(&env.db).await;

    let (vendors, total) = env
        .services
        .plugins
        .get_plugin_vendors(
            Some(10),
            Some(0),
            Some("vendor".to_string()),
            Some(false),
            ProjectScope::Active,
        )
        .await
        .unwrap();

    assert!(total >= 0);
    assert_eq!(vendors.len() as i32, total.min(10));
    for vendor in &vendors {
        assert!(!vendor.vendor.is_empty());
        assert!(vendor.plugin_count >= 0);
        assert!(vendor.installed_plugins >= 0);
        assert!(vendor.missing_plugins >= 0);
        assert!(vendor.unknown_plugins >= 0);
        assert!(vendor.total_usage_count >= 0);
        assert!(vendor.unique_projects_using >= 0);
        // `unknown` is the never-scanned rows (ADR-0012); leaving it out silently
        // loses them.
        assert_eq!(
            vendor.plugin_count,
            vendor.installed_plugins + vendor.missing_plugins + vendor.unknown_plugins
        );
    }
}

#[tokio::test]
async fn plugin_formats_have_consistent_counts() {
    let env = test_env();
    create_test_project_in_db(&env.db).await;

    let (formats, total) = env
        .services
        .plugins
        .get_plugin_formats(
            Some(10),
            Some(0),
            Some("format".to_string()),
            Some(false),
            ProjectScope::Active,
        )
        .await
        .unwrap();

    assert!(total >= 0);
    assert_eq!(formats.len() as i32, total.min(10));
    for format in &formats {
        assert!(!format.format.is_empty());
        assert!(format.plugin_count >= 0);
        assert!(format.installed_plugins >= 0);
        assert!(format.missing_plugins >= 0);
        assert!(format.unknown_plugins >= 0);
        assert!(format.total_usage_count >= 0);
        assert!(format.unique_projects_using >= 0);
        assert_eq!(
            format.plugin_count,
            format.installed_plugins + format.missing_plugins + format.unknown_plugins
        );
    }
}

/// The service answers "no such plugin" with `None`; turning that into a 404 is the
/// adapter's job.
#[tokio::test]
async fn get_plugin_finds_nothing_for_an_unknown_id() {
    let env = test_env();

    for id in ["invalid-uuid", "550e8400-e29b-41d4-a716-446655440000"] {
        let found = env
            .services
            .plugins
            .get_plugin(id, ProjectScope::Active)
            .await
            .unwrap();
        assert!(found.is_none(), "{id} should not exist");
    }
}

/// Every filter on its own and together. Empty database, so this checks that the
/// combinations are accepted and consistent, not what they select.
#[tokio::test]
async fn get_all_plugins_accepts_every_filter() {
    let env = test_env();
    create_test_project_in_db(&env.db).await;
    let plugins = &env.services.plugins;

    let (rows, total) = plugins
        .get_all_plugins(
            Some(10),
            Some(0),
            Some("name".to_string()),
            Some(false),
            Some("TestVendor".to_string()),
            Some("VST3AudioFx".to_string()),
            &[InstallState::Installed],
            Some(1),
            ProjectScope::Active,
        )
        .await
        .unwrap();
    assert!(total >= 0);
    assert_eq!(rows.len() as i32, total.min(10));

    let cases: [(
        &str,
        Option<String>,
        Option<String>,
        &[InstallState],
        Option<i32>,
        bool,
    ); 4] = [
        (
            "vendor",
            Some("AnotherVendor".into()),
            None,
            &[],
            None,
            false,
        ),
        ("format", None, Some("VST2AudioFx".into()), &[], None, false),
        // The union of everything not confirmed present.
        (
            "name",
            None,
            None,
            &[InstallState::Absent, InstallState::Unscanned],
            None,
            false,
        ),
        ("usage_count", None, None, &[], Some(5), true),
    ];
    for (sort_by, vendor, format, states, min_usage, desc) in cases {
        let (rows, total) = plugins
            .get_all_plugins(
                Some(5),
                Some(0),
                Some(sort_by.to_string()),
                Some(desc),
                vendor,
                format,
                states,
                min_usage,
                ProjectScope::Active,
            )
            .await
            .unwrap();
        assert!(total >= 0);
        assert!(rows.len() as i32 <= 5);
    }
}

/// A real refresh loads third-party plugin binaries, so the test environment's two
/// failures are both acceptable: no usable configuration, and no scanner sidecar
/// (the test harness runs from `target/debug/deps/` while `vst-meta` sits in
/// `target/debug/`). Making the sidecar findable from test binaries would mean this
/// test kicked off a real scan, minutes of loading third-party code, to check some
/// plumbing. `tests/database/plugin_scan.rs` covers the persistence logic directly.
#[tokio::test]
async fn refresh_plugin_installation_status_reports_consistent_counts_or_a_known_failure() {
    let env = test_env();

    match env
        .services
        .plugins
        .refresh_plugin_installation_status(ScanMode::Changes)
        .await
    {
        Ok(result) => {
            assert!(result.candidates_scanned >= 0);
            assert!(result.plugins_installed >= 0);
            assert!(result.plugins_missing >= 0);
            assert!(result.plugins_reconciled >= 0);
            assert!(result.scan_failures >= 0);
            // Every candidate either yielded plugins or failed to load.
            assert!(
                result.plugins_installed + result.scan_failures >= result.candidates_scanned,
                "every scanned candidate should be accounted for: {} scanned, {} installed, {} failed",
                result.candidates_scanned,
                result.plugins_installed,
                result.scan_failures
            );
        }
        Err(e) => {
            let message = e.to_string();
            let known = matches!(e, DatabaseError::ConfigError(_))
                || message.contains("ConfigError")
                || message.contains("InvalidValue")
                || message.contains("At least one path must be specified")
                || message.contains("Could not find the plugin scanner binary");
            assert!(known, "Unexpected error: {:?}", e);
        }
    }
}
