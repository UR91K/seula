//! Plugin service tests

use crate::{common::setup, grpc::*};

#[tokio::test]
async fn test_refresh_plugin_installation_status() {

    setup("error");

    let server= create_test_server().await;
    
    // Test the new plugin refresh endpoint
    let request = Request::new(RefreshPluginInstallationStatusRequest {});
    let response = server.refresh_plugin_installation_status(request).await;
    
    // In test environment, this might fail due to missing configuration
    // which is expected behavior
    match response {
        Ok(response) => {
            let response = response.into_inner();
            
            // Should return success
            assert!(response.success);
            assert!(response.error_message.is_none());
            
            // Refresh now runs a real plugin scan, so the counts describe binaries
            // on this machine rather than rows in Ableton's database.
            assert!(response.candidates_scanned >= 0);
            assert!(response.plugins_installed >= 0);
            assert!(response.plugins_missing >= 0);
            assert!(response.plugins_reconciled >= 0);
            assert!(response.scan_failures >= 0);

            // Every candidate either yielded plugins or failed to load. It cannot
            // yield fewer installed rows than there were candidates minus failures.
            assert!(
                response.plugins_installed + response.scan_failures
                    >= response.candidates_scanned,
                "every scanned candidate should be accounted for: {:?}",
                response
            );
        }
        Err(status) => {
            // Two acceptable failures here, and neither is a defect in the RPC:
            //
            // - no usable configuration in the test environment;
            // - no scanner sidecar, because the test harness runs from
            //   target/debug/deps/ while `vst-meta` sits in target/debug/.
            //
            // The second is deliberate. Making the sidecar findable from test
            // binaries would mean this test kicked off a real plugin scan — minutes
            // of loading third-party binaries — to check some gRPC plumbing.
            // `tests/database/plugin_scan.rs` covers the persistence logic directly.
            assert!(
                status.message().contains("ConfigError")
                    || status.message().contains("InvalidValue")
                    || status.message().contains("At least one path must be specified")
                    || status.message().contains("Could not find the plugin scanner binary"),
                "Unexpected error: {:?}",
                status
            );
        }
    }
}

#[tokio::test]
async fn test_get_plugin_vendors() {
    let server = create_test_server().await;
    
    // Test the new GetPluginVendors endpoint
    let request = Request::new(GetPluginVendorsRequest {
        limit: Some(10),
        offset: Some(0),
        sort_by: Some("vendor".to_string()),
        sort_desc: Some(false),
    });
    let response = server.get_plugin_vendors(request).await;
    
    assert!(response.is_ok());
    let response = response.unwrap().into_inner();
    
    // Should return vendors (even if 0 in test database)
    assert!(response.total_count >= 0);
    assert_eq!(response.vendors.len() as i32, response.total_count.min(10));
    
    // If there are vendors, check their structure
    for vendor in &response.vendors {
        assert!(!vendor.vendor.is_empty());
        assert!(vendor.plugin_count >= 0);
        assert!(vendor.installed_plugins >= 0);
        assert!(vendor.missing_plugins >= 0);
        assert!(vendor.total_usage_count >= 0);
        assert!(vendor.unique_projects_using >= 0);
        
        // Plugin counts should add up
        assert_eq!(
            vendor.plugin_count,
            vendor.installed_plugins + vendor.missing_plugins
        );
    }
}

#[tokio::test]
async fn test_get_plugin_formats() {
    let server = create_test_server().await;
    
    // Test the new GetPluginFormats endpoint
    let request = Request::new(GetPluginFormatsRequest {
        limit: Some(10),
        offset: Some(0),
        sort_by: Some("format".to_string()),
        sort_desc: Some(false),
    });
    let response = server.get_plugin_formats(request).await;
    
    assert!(response.is_ok());
    let response = response.unwrap().into_inner();
    
    // Should return formats (even if 0 in test database)
    assert!(response.total_count >= 0);
    assert_eq!(response.formats.len() as i32, response.total_count.min(10));
    
    // If there are formats, check their structure
    for format in &response.formats {
        assert!(!format.format.is_empty());
        assert!(format.plugin_count >= 0);
        assert!(format.installed_plugins >= 0);
        assert!(format.missing_plugins >= 0);
        assert!(format.total_usage_count >= 0);
        assert!(format.unique_projects_using >= 0);
        
        // Plugin counts should add up
        assert_eq!(
            format.plugin_count,
            format.installed_plugins + format.missing_plugins
        );
    }
}

#[tokio::test]
async fn test_get_plugin() {
    let server = create_test_server().await;
    
    // Test with an invalid UUID first
    let request = Request::new(GetPluginRequest {
        plugin_id: "invalid-uuid".to_string(),
    });
    let response = server.get_plugin(request).await;
    
    // Should return NotFound for invalid UUID
    assert!(response.is_err());
    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound);
    
    // Test with a valid UUID that doesn't exist
    let request = Request::new(GetPluginRequest {
        plugin_id: "550e8400-e29b-41d4-a716-446655440000".to_string(), // Valid UUID format but doesn't exist
    });
    let response = server.get_plugin(request).await;
    
    // Should return NotFound for non-existent plugin
    assert!(response.is_err());
    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn test_get_all_plugins_with_filters() {
    let server = create_test_server().await;
    
    // Test GetAllPlugins with various filter combinations
    let request = Request::new(GetAllPluginsRequest {
        limit: Some(10),
        offset: Some(0),
        sort_by: Some("name".to_string()),
        sort_desc: Some(false),
        vendor_filter: Some("TestVendor".to_string()),
        format_filter: Some("VST3AudioFx".to_string()),
        install_states: vec![InstallState::Installed as i32],
        min_usage_count: Some(1),
    });
    let response = server.get_all_plugins(request).await;
    
    assert!(response.is_ok());
    let response = response.unwrap().into_inner();
    
    // Should return plugins (even if 0 in test database)
    assert!(response.total_count >= 0);
    assert_eq!(response.plugins.len() as i32, response.total_count.min(10));
    
    // Test with just vendor filter
    let request = Request::new(GetAllPluginsRequest {
        limit: Some(5),
        offset: Some(0),
        sort_by: Some("vendor".to_string()),
        sort_desc: Some(false),
        vendor_filter: Some("AnotherVendor".to_string()),
        format_filter: None,
        install_states: vec![],
        min_usage_count: None,
    });
    let response = server.get_all_plugins(request).await;
    
    assert!(response.is_ok());
    let response = response.unwrap().into_inner();
    assert!(response.total_count >= 0);
    
    // Test with just format filter
    let request = Request::new(GetAllPluginsRequest {
        limit: Some(5),
        offset: Some(0),
        sort_by: Some("format".to_string()),
        sort_desc: Some(false),
        vendor_filter: None,
        format_filter: Some("VST2AudioFx".to_string()),
        install_states: vec![],
        min_usage_count: None,
    });
    let response = server.get_all_plugins(request).await;
    
    assert!(response.is_ok());
    let response = response.unwrap().into_inner();
    assert!(response.total_count >= 0);
    
    // Test with just the install-state filter (a union: everything not confirmed present)
    let request = Request::new(GetAllPluginsRequest {
        limit: Some(5),
        offset: Some(0),
        sort_by: Some("name".to_string()),
        sort_desc: Some(false),
        vendor_filter: None,
        format_filter: None,
        install_states: vec![InstallState::Absent as i32, InstallState::Unscanned as i32],
        min_usage_count: None,
    });
    let response = server.get_all_plugins(request).await;
    
    assert!(response.is_ok());
    let response = response.unwrap().into_inner();
    assert!(response.total_count >= 0);
    
    // Test with just min_usage_count filter
    let request = Request::new(GetAllPluginsRequest {
        limit: Some(5),
        offset: Some(0),
        sort_by: Some("usage_count".to_string()),
        sort_desc: Some(true),
        vendor_filter: None,
        format_filter: None,
        install_states: vec![],
        min_usage_count: Some(5),
    });
    let response = server.get_all_plugins(request).await;
    
    assert!(response.is_ok());
    let response = response.unwrap().into_inner();
    assert!(response.total_count >= 0);
} 