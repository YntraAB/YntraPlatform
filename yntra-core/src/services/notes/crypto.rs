use crate::database;
use crate::infra::errors::YntraError;

pub async fn verify_zkp_if_encrypted(
    conn: &database::DbConnection,
    content: &str,
    user_id: &str,
    role: &str,
    workspace_id: &str,
    team_id: &str,
) -> Result<(), YntraError> {
    if content.starts_with("zero_copy_enc:") {
        let rest = &content["zero_copy_enc:".len()..];
        let (proof, ciphertext) = match rest.split_once(':') {
            Some((p, c)) => (p, c),
            None => {
                return Err(YntraError::CryptoError(
                    "Invalid encrypted payload format".to_string(),
                ));
            }
        };
        let trust = crate::ZkCryptoTrust::new();
        let ciphertext_bytes = match const_hex::decode(ciphertext) {
            Ok(b) => b,
            Err(_) => ciphertext.as_bytes().to_vec(),
        };
        let data_hash = blake3::hash(&ciphertext_bytes);
        let data_hash_hex = const_hex::encode(data_hash.as_bytes());

        let public_key_hex: String;

        let is_ring = if let Ok(proof_bytes) = const_hex::decode(proof) {
            proof_bytes.starts_with(b"ZKP_RING_PROOF_V1:")
        } else {
            false
        };

        if is_ring {
            // Query all candidate authorized users' public keys to form the ring
            let mut stmt_candidates = conn.prepare(
                "SELECT id FROM users WHERE id = ?1 \
                 UNION \
                 SELECT u.id FROM team_members tm JOIN users u ON tm.user_id = u.id WHERE tm.team_id = ?2 \
                 UNION \
                 SELECT id FROM users WHERE role = 'platform_admin' OR (role = 'admin' AND workspace_id = ?3)"
            ).await?;
            let mut rows = stmt_candidates
                .query(crate::params![user_id, team_id, workspace_id])
                .await?;
            let mut pks = Vec::new();
            while let Some(row) = rows.next().await? {
                let uid: String = row.get(0)?;
                let metadata_str: Option<String> = conn
                    .query_row(
                        "SELECT metadata FROM users WHERE id = ?1",
                        crate::params![&uid],
                        |r| r.get(0),
                    )
                    .await
                    .ok()
                    .flatten();
                if let Some(ref meta) = metadata_str {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(meta) {
                        let pk = val
                            .get("public_key")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .or_else(|| {
                                val.get("siths_public_key")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                            });
                        if let Some(p) = pk {
                            pks.push(p);
                        }
                    }
                }
            }
            pks.sort();
            pks.dedup();
            public_key_hex = pks.join(",");
        } else {
            // Standard single key verification
            let metadata_str: Option<String> = conn
                .query_row(
                    "SELECT metadata FROM users WHERE id = ?1",
                    crate::params![user_id],
                    |r| r.get(0),
                )
                .await
                .ok()
                .flatten();

            public_key_hex = if let Some(ref meta) = metadata_str {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(meta) {
                    val.get("public_key")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            val.get("siths_public_key")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        })
                } else {
                    None
                }
            } else {
                None
            }
            .unwrap_or_default();
        }

        if !trust
            .verify_compliance_proof(
                proof.to_string(),
                user_id.to_string(),
                role.to_string(),
                data_hash_hex,
                public_key_hex,
            )
            .unwrap_or(false)
        {
            return Err(YntraError::CryptoError(
                "Validation failed: Zero-Knowledge compliance proof is invalid".to_string(),
            ));
        }
    }
    Ok(())
}
