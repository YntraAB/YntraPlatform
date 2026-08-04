use crate::database;
use crate::services::jobs::routing::{geocode, mock_geocode};
use crate::services::jobs::{
    create_job_ticket, get_directions_url, get_job_tickets, optimize_job_route, update_route_stops,
};

#[tokio::test]
async fn test_gps_routing_urls() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // Setup test workspace and users
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-gps-test', 'GPS Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-gps-staff', 'ws-gps-test', 'staff@gps.io', 'admin')", ()).await.unwrap();

    // 1. Create job ticket with both origin and destination addresses
    let job1 = create_job_ticket(
        "u-gps-staff".to_string(),
        "ws-gps-test".to_string(),
        "Cabinet relocation".to_string(),
        "Delicate office cabinets".to_string(),
        "Dest Road 10".to_string(),
        "medium".to_string(),
        None,
        "2026-08-14".to_string(),
        "[]".to_string(),
        Some("Origin St 1".to_string()),
        Some("Dest St 5".to_string()),
        0,
        0,
        false,
        false,
        false,
        false,
    )
    .await
    .unwrap();

    // Verify routing URL with both origin and destination
    let url1 = get_directions_url("u-gps-staff".to_string(), job1.id.clone())
        .await
        .unwrap();
    assert_eq!(
        url1,
        "https://www.google.com/maps/dir/?api=1&origin=Origin%20St%201&destination=Dest%20St%205&travelmode=truck&dirflg=t"
    );

    // 2. Create job ticket with destination only (relying on fallback to location_address)
    let job2 = create_job_ticket(
        "u-gps-staff".to_string(),
        "ws-gps-test".to_string(),
        "Cabinet relocation".to_string(),
        "Delicate office cabinets".to_string(),
        "Location St 20".to_string(),
        "medium".to_string(),
        None,
        "2026-08-14".to_string(),
        "[]".to_string(),
        None,
        None,
        0,
        0,
        false,
        false,
        false,
        false,
    )
    .await
    .unwrap();

    let url2 = get_directions_url("u-gps-staff".to_string(), job2.id.clone())
        .await
        .unwrap();
    assert_eq!(
        url2,
        "https://www.google.com/maps/dir/?api=1&destination=Location%20St%2020&travelmode=truck&dirflg=t"
    );

    // Cleanup
    conn.execute(
        "DELETE FROM job_tickets WHERE workspace_id = 'ws-gps-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-gps-test'", ())
        .await
        .unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-gps-test'", ())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_country_aware_geocoding_fallbacks() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Test US company fallback (San Francisco region)
    let settings_us = serde_json::json!({
        "company_country": "US",
        "geocoder_provider": "nominatim",
        "geocoder_url": "http://invalid.local",
    })
    .to_string();
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-geo-us', 'US WS', '[\"moving_company\"]', ?1)", crate::params![&settings_us]).await.unwrap();

    let coords_us = geocode("ws-geo-us", "123 Main St").await;
    // San Francisco lat ~37.77, should be within standard hash range
    assert!((coords_us.0 - 37.7749).abs() < 0.5);

    // 2. Test DE company fallback (Berlin region)
    let settings_de = serde_json::json!({
        "company_country": "DE",
        "geocoder_provider": "nominatim",
        "geocoder_url": "http://invalid.local",
    })
    .to_string();
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-geo-de', 'DE WS', '[\"moving_company\"]', ?1)", crate::params![&settings_de]).await.unwrap();

    let coords_de = geocode("ws-geo-de", "123 Main St").await;
    // Berlin lat ~52.52
    assert!((coords_de.0 - 52.5200).abs() < 0.5);

    // Cleanup
    conn.execute(
        "DELETE FROM workspaces WHERE id IN ('ws-geo-us', 'ws-geo-de')",
        (),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_multi_stop_route_optimization() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup workspace & user
    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-route-test', 'Route Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-route-staff', 'ws-route-test', 'staff@route.io', 'admin')", ()).await.unwrap();

    // 2. Create job ticket
    let job = create_job_ticket(
        "u-route-staff".to_string(),
        "ws-route-test".to_string(),
        "Relocation route".to_string(),
        "Testing TSP optimization".to_string(),
        "Recycling Center".to_string(),
        "medium".to_string(),
        None,
        "2026-08-15".to_string(),
        "[]".to_string(),
        Some("Warehouse".to_string()),
        Some("Drop-off B".to_string()),
        0,
        0,
        false,
        false,
        false,
        false,
    )
    .await
    .unwrap();

    // Verify initial stops
    assert_eq!(job.route_stops_json, Some("[]".to_string()));

    // 3. Update route stops
    let stops = vec!["Recycling Center".to_string(), "Pickup A".to_string()];
    update_route_stops("u-route-staff".to_string(), job.id.clone(), stops.clone())
        .await
        .unwrap();

    // Reload job and check stops
    let tickets = get_job_tickets("u-route-staff".to_string()).await.unwrap();
    let reloaded = tickets.iter().find(|t| t.id == job.id).unwrap();
    assert_eq!(
        reloaded.route_stops_json.as_deref(),
        Some("[\"Recycling Center\",\"Pickup A\"]")
    );

    // 4. Optimize stops
    let optimized = optimize_job_route("u-route-staff".to_string(), job.id.clone())
        .await
        .unwrap();

    assert_eq!(optimized.len(), 2);
    let tickets2 = get_job_tickets("u-route-staff".to_string()).await.unwrap();
    let reloaded2 = tickets2.iter().find(|t| t.id == job.id).unwrap();
    let reloaded_stops: Vec<String> =
        serde_json::from_str(reloaded2.route_stops_json.as_deref().unwrap()).unwrap();
    assert_eq!(reloaded_stops, optimized);

    // 5. Directions URL verification
    let directions_url = get_directions_url("u-route-staff".to_string(), job.id.clone())
        .await
        .unwrap();

    assert!(directions_url.contains("origin=Warehouse"));
    assert!(directions_url.contains("destination=Drop-off%20B"));
    assert!(directions_url.contains("waypoints="));
    assert!(directions_url.contains("Recycling%20Center"));
    assert!(directions_url.contains("Pickup%20A"));

    // Cleanup
    conn.execute(
        "DELETE FROM job_tickets WHERE workspace_id = 'ws-route-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-route-test'", ())
        .await
        .unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-route-test'", ())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_real_time_geocoding_with_fallback() {
    // Test with Stockholm (this should resolve via Nominatim if online, or fall back to mock)
    let coords = geocode("ws-route-test", "Stockholm").await;
    assert!(coords.0 != 0.0);
    assert!(coords.1 != 0.0);

    // Test with empty string
    let coords_empty = geocode("ws-route-test", "").await;
    assert_eq!(coords_empty, (0.0, 0.0));

    // Test that fallback works for a random address string
    let _mock_coords = mock_geocode("Random non-existent address 12345");
    let coords_fallback = geocode("ws-route-test", "Random non-existent address 12345").await;

    // It should either resolve to actual coords or fall back to mock coords
    assert!(coords_fallback.0 != 0.0);
}

#[tokio::test]
async fn test_geocoding_provider_selection() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    // 1. Setup mock workspace with custom geocoding settings
    conn.execute(
        "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-geo-test', 'Geo Test WS', '[]', '{\"geocoder_provider\":\"photon\",\"geocoder_url\":\"http://localhost:9999\"}')",
        ()
    ).await.unwrap();

    // 2. Call geocode which should read settings and fail/fallback since http://localhost:9999 is down
    let coords = geocode("ws-geo-test", "Stockholm").await;
    // Fallback coordinates should still be generated
    assert!(coords.0 != 0.0);
    assert!(coords.1 != 0.0);

    conn.execute("DELETE FROM workspaces WHERE id = 'ws-geo-test'", ())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_commercial_truck_routing_and_restrictions() {
    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    let conn = database::acquire_connection().await.unwrap();

    use crate::services::jobs::{
        get_commercial_truck_directions_url, verify_commercial_route_restrictions,
    };

    conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-truck-test', 'Truck Test WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
    conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-truck-staff', 'ws-truck-test', 'staff@truck.io', 'admin')", ()).await.unwrap();

    let job = create_job_ticket(
        "u-truck-staff".to_string(),
        "ws-truck-test".to_string(),
        "Heavy Transport Stockholm".to_string(),
        "Office relocation".to_string(),
        "Vasagatan 10, Stockholm".to_string(),
        "high".to_string(),
        None,
        "2026-08-20".to_string(),
        "[]".to_string(),
        Some("Kungsgatan 2, Stockholm".to_string()),
        Some("Vasagatan 10, Stockholm".to_string()),
        0,
        0,
        true,
        true,
        true,
        true,
    )
    .await
    .unwrap();

    // 1. Verify commercial heavy truck directions URL
    let truck_url = get_commercial_truck_directions_url(
        "u-truck-staff".to_string(),
        job.id.clone(),
        Some(4.1),
        Some(18.0),
        Some("google_truck".to_string()),
    )
    .await
    .unwrap();

    assert!(truck_url.contains("travelmode=truck"));
    assert!(truck_url.contains("dirflg=t"));
    assert!(truck_url.contains("origin=Kungsgatan%202%2C%20Stockholm"));
    assert!(truck_url.contains("destination=Vasagatan%2010%2C%20Stockholm"));

    // 2. Verify commercial route restriction checks
    let restrictions = verify_commercial_route_restrictions(
        "u-truck-staff".to_string(),
        job.id.clone(),
        4.1,                         // 4.1m height (> 3.8m limit) -> low bridge warning
        18.0,                        // 18.0t weight (> 3.5t limit) -> weight limit warning
        "Euro 4 Diesel".to_string(), // Euro 4 Diesel in Stockholm -> environmental zone warning
    )
    .await
    .unwrap();

    assert!(restrictions.low_bridge_warning);
    assert!(restrictions.environmental_zone_warning);
    assert!(restrictions.weight_limit_warning);
    assert!(restrictions.parking_permit_required);
    assert!(restrictions.restriction_details.len() >= 4);

    conn.execute(
        "DELETE FROM job_tickets WHERE workspace_id = 'ws-truck-test'",
        (),
    )
    .await
    .unwrap();
    conn.execute("DELETE FROM users WHERE workspace_id = 'ws-truck-test'", ())
        .await
        .unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = 'ws-truck-test'", ())
        .await
        .unwrap();
}
