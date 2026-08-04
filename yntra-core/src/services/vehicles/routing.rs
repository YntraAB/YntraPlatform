use crate::YntraError;
use crate::database;

#[uniffi::export]
pub async fn get_vehicle_commercial_routing_profile(
    requester_user_id: String,
    vehicle_id: String,
) -> Result<crate::CommercialRouteRestrictions, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (ws_id, capacity_m3): (String, f64) = conn
        .query_row(
            "SELECT workspace_id, capacity_m3 FROM vehicles WHERE id = ?1",
            crate::params![&vehicle_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Vehicle not found".to_string()))?;

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let estimated_height = if capacity_m3 > 35.0 {
        3.9
    } else if capacity_m3 > 15.0 {
        3.4
    } else {
        2.6
    };
    let estimated_weight = if capacity_m3 > 35.0 {
        16.0
    } else if capacity_m3 > 15.0 {
        7.5
    } else {
        3.5
    };

    let mut details = Vec::new();
    let low_bridge_warning = estimated_height >= 3.8;
    let weight_limit_warning = estimated_weight >= 3.5;

    if low_bridge_warning {
        details.push(format!(
            "Vehicle height ({:.1}m) requires commercial truck navigation route planning.",
            estimated_height
        ));
    }
    if weight_limit_warning {
        details.push(format!(
            "Vehicle weight ({:.1}t) requires residential weight restriction checks.",
            estimated_weight
        ));
    }

    Ok(crate::CommercialRouteRestrictions {
        low_bridge_warning,
        environmental_zone_warning: false,
        weight_limit_warning,
        parking_permit_required: capacity_m3 > 20.0,
        restriction_details: details,
    })
}

#[uniffi::export]
pub async fn evaluate_vehicle_route_clearance(
    requester_user_id: String,
    vehicle_id: String,
    origin_address: String,
    destination_address: String,
) -> Result<crate::CommercialRouteRestrictions, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (ws_id, capacity_m3): (String, f64) = conn
        .query_row(
            "SELECT workspace_id, capacity_m3 FROM vehicles WHERE id = ?1",
            crate::params![&vehicle_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Vehicle not found".to_string()))?;

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let estimated_height = if capacity_m3 > 35.0 {
        3.9
    } else if capacity_m3 > 15.0 {
        3.4
    } else {
        2.6
    };
    let estimated_weight = if capacity_m3 > 35.0 {
        16.0
    } else if capacity_m3 > 15.0 {
        7.5
    } else {
        3.5
    };
    let emission_class = if capacity_m3 > 35.0 {
        "Euro 6 Heavy Diesel"
    } else {
        "Euro 6 Clean"
    };

    let combined = format!("{} {}", origin_address, destination_address).to_lowercase();

    let mut details = Vec::new();
    let mut low_bridge_warning = false;
    let mut weight_limit_warning = false;
    let mut environmental_zone_warning = false;

    if estimated_height >= 3.8 {
        low_bridge_warning = true;
        details.push(format!(
            "Low bridge risk: Heavy truck height {:.1}m exceeds 3.8m standard urban clearance.",
            estimated_height
        ));
    }

    if estimated_weight >= 3.5 {
        weight_limit_warning = true;
        details.push(format!(
            "Weight limit warning: {:.1}t vehicle exceeds 3.5t residential zone limit.",
            estimated_weight
        ));
    }

    let env_cities = [
        "stockholm",
        "göteborg",
        "gothenburg",
        "malmö",
        "malmo",
        "berlin",
        "london",
        "paris",
        "hamburg",
    ];
    if env_cities.iter().any(|c| combined.contains(c)) {
        environmental_zone_warning = true;
        details.push(format!("Low Emission Zone (LEZ) warning: Target city enforces Euro 6 / Green badge regulations. Vehicle class: '{}'.", emission_class));
    }

    let parking_permit_required = capacity_m3 > 20.0;
    if parking_permit_required {
        details.push(
            "Commercial loading zone parking permit recommended for target addresses.".to_string(),
        );
    }

    Ok(crate::CommercialRouteRestrictions {
        low_bridge_warning,
        environmental_zone_warning,
        weight_limit_warning,
        parking_permit_required,
        restriction_details: details,
    })
}
