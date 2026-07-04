use serde_json::Value;

// Embed Swedish assets
const SCHOOL_SV: &str = include_str!("../../assets/templates/school_sv.json");
const MOVING_SV: &str = include_str!("../../assets/templates/moving_sv.json");
const CARE_LSS_SV: &str = include_str!("../../assets/templates/care_lss_sv.json");
const CARE_HVB_SV: &str = include_str!("../../assets/templates/care_hvb_sv.json");
const CARE_GENERAL_SV: &str = include_str!("../../assets/templates/care_general_sv.json");

// Embed English assets
const SCHOOL_EN: &str = include_str!("../../assets/templates/school_en.json");
const MOVING_EN: &str = include_str!("../../assets/templates/moving_en.json");
const CARE_LSS_EN: &str = include_str!("../../assets/templates/care_lss_en.json");
const CARE_HVB_EN: &str = include_str!("../../assets/templates/care_hvb_en.json");
const CARE_GENERAL_EN: &str = include_str!("../../assets/templates/care_general_en.json");

fn parse_template(json_str: &str) -> Value {
    serde_json::from_str(json_str).unwrap_or_else(|_| serde_json::Value::Array(Vec::new()))
}

pub fn get_school_roles(is_scandi: bool) -> Value {
    if is_scandi {
        parse_template(SCHOOL_SV)
    } else {
        parse_template(SCHOOL_EN)
    }
}

pub fn get_care_roles(care_subtype: &str, is_scandi: bool) -> Value {
    if is_scandi {
        match care_subtype {
            "lss" => parse_template(CARE_LSS_SV),
            "hvb" => parse_template(CARE_HVB_SV),
            _ => parse_template(CARE_GENERAL_SV),
        }
    } else {
        match care_subtype {
            "lss" => parse_template(CARE_LSS_EN),
            "hvb" => parse_template(CARE_HVB_EN),
            _ => parse_template(CARE_GENERAL_EN),
        }
    }
}

pub fn get_moving_company_roles(is_scandi: bool) -> Value {
    if is_scandi {
        parse_template(MOVING_SV)
    } else {
        parse_template(MOVING_EN)
    }
}

#[uniffi::export]
pub fn get_default_roles_json(
    workspace_type: String,
    care_subtype: Option<String>,
    is_scandi: bool,
) -> Result<String, crate::YntraError> {
    let val = match workspace_type.as_str() {
        "school" => get_school_roles(is_scandi),
        "moving_company" => get_moving_company_roles(is_scandi),
        "assistance" => {
            let subtype = care_subtype.as_deref().unwrap_or("aldreomsorg");
            get_care_roles(subtype, is_scandi)
        }
        _ => return Err(crate::YntraError::NotFoundError(format!("Unknown workspace type: {}", workspace_type))),
    };

    serde_json::to_string(&val)
        .map_err(|e| crate::YntraError::DbError(format!("Failed to serialize role template: {}", e)))
}
