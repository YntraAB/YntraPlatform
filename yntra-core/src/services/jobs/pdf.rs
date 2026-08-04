use crate::database;
use crate::infra::errors::YntraError;

#[uniffi::export]
pub async fn generate_printable_bol_html(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let bol_res =
        crate::services::jobs::signatures::get_bill_of_lading(job_ticket_id.clone()).await?;
    let bol = bol_res
        .ok_or_else(|| YntraError::NotFoundError("Bill of Lading not found for job".to_string()))?;

    if auth.workspace_id != bol.workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let usdot = bol
        .carrier_dot_number
        .as_deref()
        .unwrap_or("Pending / Registered");
    let orig_sig = bol
        .origin_signature_hash
        .as_deref()
        .unwrap_or("Unsigned / Pending");
    let dest_sig = bol
        .destination_signature_hash
        .as_deref()
        .unwrap_or("Unsigned / Pending");

    let val_label = if bol.valuation_option == "released_value_060" {
        "Released Value Protection ($0.60/lb per article - STB Default)"
    } else {
        "Full Value Protection (Full Replacement Value)"
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Bill of Lading - {}</title>
<style>
  @page {{ size: letter portrait; margin: 0.5in; }}
  body {{ font-family: 'Helvetica Neue', Arial, sans-serif; color: #1e293b; line-height: 1.5; margin: 0; padding: 20px; background: #fff; }}
  .header {{ display: flex; justify-content: space-between; align-items: flex-start; border-b: 2px solid #0f172a; padding-bottom: 12px; margin-bottom: 20px; }}
  .title {{ font-size: 22px; font-weight: 800; color: #0f172a; margin: 0; text-transform: uppercase; letter-spacing: 0.5px; }}
  .subtitle {{ font-size: 11px; color: #64748b; margin-top: 4px; }}
  .badge {{ background: #e2e8f0; color: #0f172a; font-size: 11px; font-weight: 700; padding: 4px 8px; border-radius: 4px; display: inline-block; }}
  .grid {{ display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-bottom: 20px; }}
  .card {{ border: 1px solid #cbd5e1; border-radius: 6px; padding: 12px; background: #f8fafc; }}
  .card-title {{ font-size: 10px; font-weight: 700; color: #64748b; text-transform: uppercase; margin-bottom: 6px; }}
  .card-val {{ font-size: 13px; font-weight: 600; color: #0f172a; }}
  .terms-box {{ border: 1px solid #cbd5e1; background: #f1f5f9; padding: 12px; border-radius: 6px; font-size: 10px; font-family: monospace; color: #334155; margin-bottom: 20px; max-height: 100px; overflow: hidden; }}
  .sig-grid {{ display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-top: 20px; }}
  .sig-card {{ border: 1px dashed #94a3b8; border-radius: 6px; padding: 12px; text-align: center; }}
  .sig-label {{ font-size: 11px; font-weight: 700; color: #475569; uppercase; }}
  .sig-hash {{ font-size: 9px; font-family: monospace; color: #16a34a; margin-top: 6px; word-break: break-all; }}
  .footer {{ margin-top: 30px; border-t: 1px solid #e2e8f0; pt: 10px; font-size: 10px; color: #94a3b8; text-align: center; }}
</style>
</head>
<body>
  <div class="header">
    <div>
      <h1 class="title">Bill of Lading (BOL)</h1>
      <div class="subtitle">FMCSA / STB Carmack Amendment Compliant Contract of Carriage (49 U.S.C. § 14706)</div>
    </div>
    <div style="text-align: right;">
      <div class="badge">BOL #: {}</div>
      <div style="font-size: 11px; color: #475569; margin-top: 4px;">USDOT #: {}</div>
    </div>
  </div>

  <div class="grid">
    <div class="card">
      <div class="card-title">Carrier Information</div>
      <div class="card-val">{}</div>
    </div>
    <div class="card">
      <div class="card-title">Shipper / Customer Name</div>
      <div class="card-val">{}</div>
    </div>
    <div class="card">
      <div class="card-title">Origin Pickup Address</div>
      <div class="card-val">{}</div>
    </div>
    <div class="card">
      <div class="card-title">Destination Delivery Address</div>
      <div class="card-val">{}</div>
    </div>
  </div>

  <div class="card" style="margin-bottom: 20px;">
    <div class="card-title">Carrier Liability & Valuation Coverage</div>
    <div style="font-size: 13px; font-weight: 700; color: #2563eb; margin-bottom: 8px;">{}</div>
    <div style="display: flex; gap: 24px; font-size: 11px; color: #334155;">
      <div>Declared Protection Value: <strong>${:.2}</strong></div>
      <div>Deductible Amount: <strong>${:.2}</strong></div>
      <div>Valuation Premium: <strong>${:.2}</strong></div>
      <div>Estimated Weight: <strong>{:.0} lbs</strong></div>
    </div>
  </div>

  <div class="card-title">STB Carmack Legal Disclosures & Terms</div>
  <div class="terms-box">{}</div>

  <div class="sig-grid">
    <div class="sig-card">
      <div class="sig-label">Phase 1: Pickup / Origin Signature</div>
      <div class="sig-hash">Hash: {}</div>
    </div>
    <div class="sig-card">
      <div class="sig-label">Phase 2: Delivery / Destination Signature</div>
      <div class="sig-hash">Hash: {}</div>
    </div>
  </div>

  <div class="footer">
    Cryptographically Sealed Document Audit Hash: <strong>{}</strong> | Generated via Yntra Engine
  </div>
</body>
</html>"#,
        bol.bol_number,
        bol.bol_number,
        usdot,
        bol.carrier_name,
        bol.shipper_name,
        bol.origin_address,
        bol.destination_address,
        val_label,
        bol.valuation_declared_amount,
        bol.valuation_deductible,
        bol.valuation_premium,
        bol.total_estimated_weight_lbs,
        bol.legal_terms,
        orig_sig,
        dest_sig,
        bol.document_tamper_hash
    );

    Ok(html)
}

#[uniffi::export]
pub async fn generate_printable_invoice_html(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (ws_id, title, loc_addr): (String, String, String) = conn
        .query_row(
            "SELECT workspace_id, title, location_address FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job ticket not found".to_string()))?;

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let summary = crate::services::jobs::moves::get_move_inventory_summary(
        requester_user_id.clone(),
        job_ticket_id.clone(),
    )
    .await
    .unwrap_or(crate::MoveInventorySummary {
        total_volume_m3: 0.0,
        total_weight_kg: 0.0,
        total_volume_cu_ft: 0.0,
        total_weight_lbs: 0.0,
        total_item_count: 0,
        recommended_truck_m3: 0.0,
        recommended_truck_cu_ft: 0.0,
        recommended_crew_size: 2,
        truck_capacity_exceeded: false,
        truck_capacity_warning: None,
    });

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Move Invoice - {}</title>
<style>
  @page {{ size: letter portrait; margin: 0.5in; }}
  body {{ font-family: 'Helvetica Neue', Arial, sans-serif; color: #1e293b; line-height: 1.5; margin: 0; padding: 20px; }}
  .header {{ display: flex; justify-content: space-between; border-b: 2px solid #0f172a; padding-bottom: 12px; margin-bottom: 20px; }}
  .title {{ font-size: 22px; font-weight: 800; color: #0f172a; margin: 0; }}
  .grid {{ display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-bottom: 20px; }}
  .card {{ border: 1px solid #cbd5e1; border-radius: 6px; padding: 12px; background: #f8fafc; }}
  .card-title {{ font-size: 10px; font-weight: 700; color: #64748b; uppercase; margin-bottom: 4px; }}
  .table {{ width: 100%; border-collapse: collapse; margin-top: 16px; font-size: 12px; }}
  .table th, .table td {{ border: 1px solid #cbd5e1; padding: 8px 12px; text-align: left; }}
  .table th {{ background: #f1f5f9; font-weight: 700; text-transform: uppercase; font-size: 10px; }}
</style>
</head>
<body>
  <div class="header">
    <div>
      <h1 class="title">Official Move Invoice & Summary</h1>
      <div style="font-size: 11px; color: #64748b;">Job Ticket #: {}</div>
    </div>
    <div style="text-align: right;">
      <div style="font-size: 14px; font-weight: 800; color: #0f172a;">Yntra Platform Freight Services</div>
    </div>
  </div>

  <div class="grid">
    <div class="card">
      <div class="card-title">Customer / Job Title</div>
      <div style="font-size: 13px; font-weight: 700;">{}</div>
    </div>
    <div class="card">
      <div class="card-title">Primary Address</div>
      <div style="font-size: 13px; font-weight: 600;">{}</div>
    </div>
  </div>

  <table class="table">
    <thead>
      <tr>
        <th>Service / Manifest Metric</th>
        <th>Recorded Value</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>Total Inventory Item Count</td>
        <td><strong>{} items</strong></td>
      </tr>
      <tr>
        <td>Total Calculated Volume</td>
        <td><strong>{:.2} m³</strong></td>
      </tr>
      <tr>
        <td>Total Estimated Weight</td>
        <td><strong>{:.0} lbs ({:.1} kg)</strong></td>
      </tr>
    </tbody>
  </table>
</body>
</html>"#,
        job_ticket_id,
        job_ticket_id,
        title,
        loc_addr,
        summary.total_item_count,
        summary.total_volume_m3,
        summary.total_weight_lbs,
        summary.total_weight_kg
    );

    Ok(html)
}
