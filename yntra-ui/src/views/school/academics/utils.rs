use serde::{Deserialize, Serialize};
use dioxus::prelude::*;
use crate::components::LucideIcon;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdvancedAttachment {
    pub filename: String,
    pub size_str: String,
    pub sha256: String,
    pub e2ee: bool,
    pub dataurl: String,
}

pub fn is_deadline_passed(due_date: &str) -> bool {
    if due_date.is_empty() || due_date == "No due date" {
        return false;
    }
    if due_date.contains("(Strict)") || due_date.contains("(Flexible)") {
        if due_date.len() >= 10 {
            let date_str = &due_date[0..10];
            if let Ok(due_naive) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                let today = chrono::Utc::now().date_naive();
                return today > due_naive;
            }
        }
    }
    false
}

pub fn parse_submission_content_and_advanced_attachment(content: &str) -> (String, Option<AdvancedAttachment>) {
    if let Some(idx) = content.rfind("\n[Attachment: ") {
        let text = content[..idx].to_string();
        let link_part = &content[idx + "\n[Attachment: ".len()..];
        if let Some(end_idx) = link_part.find(']') {
            let payload = &link_part[..end_idx];
            let parts: Vec<&str> = payload.split('|').collect();
            if parts.len() >= 5 {
                return (text, Some(AdvancedAttachment {
                    filename: parts[0].trim().to_string(),
                    size_str: parts[1].trim().to_string(),
                    sha256: parts[2].trim().to_string(),
                    e2ee: parts[3].trim() == "true",
                    dataurl: parts[4].trim().to_string(),
                }));
            } else if parts.len() == 2 {
                return (text, Some(AdvancedAttachment {
                    filename: parts[0].trim().to_string(),
                    size_str: "Unknown size".to_string(),
                    sha256: "legacy-unhashed".to_string(),
                    e2ee: false,
                    dataurl: parts[1].trim().to_string(),
                }));
            }
        }
    }
    (content.to_string(), None)
}

pub fn compute_mock_hash(bytes: &[u8]) -> String {
    let mut hash = 0u64;
    for &b in bytes {
        hash = hash.wrapping_add(b as u64).wrapping_mul(31);
    }
    format!("{:x}", hash)
}

pub fn format_file_size(bytes_len: usize) -> String {
    if bytes_len >= 1_048_576 {
        format!("{:.1} MB", bytes_len as f64 / 1_048_576.0)
    } else if bytes_len >= 1024 {
        format!("{:.1} KB", bytes_len as f64 / 1024.0)
    } else {
        format!("{} B", bytes_len)
    }
}

pub fn base64_encode(bytes: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = match chunk.len() {
            3 => ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32),
            2 => ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8),
            1 => (chunk[0] as u32) << 16,
            _ => unreachable!(),
        };
        let c1 = CHARSET[((b >> 18) & 63) as usize] as char;
        let c2 = CHARSET[((b >> 12) & 63) as usize] as char;
        let c3 = if chunk.len() > 1 {
            CHARSET[((b >> 6) & 63) as usize] as char
        } else {
            '='
        };
        let c4 = if chunk.len() > 2 {
            CHARSET[(b & 63) as usize] as char
        } else {
            '='
        };
        result.push(c1);
        result.push(c2);
        result.push(c3);
        result.push(c4);
    }
    result
}

#[component]
pub fn BlobDownloadLink(
    filename: String,
    dataurl: String,
    sha256: String,
    class: String,
    children: Element,
) -> Element {
    let state = use_context::<crate::state::AppState>();
    let active_uid = state.active_user_id.read().clone();

    let mut resolved_url = use_signal(|| {
        if dataurl.starts_with("blob://") {
            String::new()
        } else {
            dataurl.clone()
        }
    });

    use_effect(move || {
        let url = dataurl.clone();
        let hash = sha256.clone();
        let uid = active_uid.clone();
        spawn(async move {
            if url.starts_with("blob://") {
                if let Ok(data) = yntra_core::get_blob(uid, hash).await {
                    resolved_url.set(data);
                }
            }
        });
    });

    if resolved_url.read().is_empty() {
        rsx! {
            span { class: "{class} opacity-50 cursor-wait flex items-center gap-1.5",
                LucideIcon { name: "loader", class: "h-3.5 w-3.5 animate-spin mr-1.5" }
                span { "{filename}" }
            }
        }
    } else {
        rsx! {
            a {
                class: "{class}",
                href: "{resolved_url}",
                download: "{filename}",
                {children}
            }
        }
    }
}
