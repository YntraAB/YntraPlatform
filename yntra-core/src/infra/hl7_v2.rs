/// MLLP Framing Characters
pub const MLLP_START_BLOCK: u8 = 0x0B; // <VT>
pub const MLLP_END_BLOCK_1: u8 = 0x1C; // <FS>
pub const MLLP_END_BLOCK_2: u8 = 0x0D; // <CR>

/// Encapsulate payload into an MLLP frame (<VT> + payload + <FS><CR>).
pub fn encode_mllp_frame(payload: &str) -> Vec<u8> {
    let mut frame = Vec::with_capacity(payload.len() + 3);
    frame.push(MLLP_START_BLOCK);
    frame.extend_from_slice(payload.as_bytes());
    frame.push(MLLP_END_BLOCK_1);
    frame.push(MLLP_END_BLOCK_2);
    frame
}

/// Decode complete MLLP frames from a byte stream buffer.
/// Extracts any fully framed messages and removes processed bytes from `buffer`.
pub fn decode_mllp_frames(buffer: &mut Vec<u8>) -> Vec<String> {
    let mut frames = Vec::new();
    loop {
        let start_pos = match buffer.iter().position(|&b| b == MLLP_START_BLOCK) {
            Some(pos) => pos,
            None => {
                buffer.clear();
                break;
            }
        };

        // Find end sequence: MLLP_END_BLOCK_1 (0x1C) followed by MLLP_END_BLOCK_2 (0x0D)
        let mut end_pos = None;
        for i in (start_pos + 1)..buffer.len() {
            if buffer[i] == MLLP_END_BLOCK_1 {
                if i + 1 < buffer.len() && buffer[i + 1] == MLLP_END_BLOCK_2 {
                    end_pos = Some(i);
                    break;
                }
            }
        }

        if let Some(end) = end_pos {
            let payload_bytes = &buffer[(start_pos + 1)..end];
            let payload_str = String::from_utf8_lossy(payload_bytes).to_string();
            frames.push(payload_str);
            // Drain buffer up to end_pos + 2
            buffer.drain(0..(end + 2));
        } else {
            // Unfinished frame, trim any garbage before start_pos and wait for more data
            if start_pos > 0 {
                buffer.drain(0..start_pos);
            }
            break;
        }
    }
    frames
}

/// HL7 v2 Message Header (MSH)
#[derive(
    uniffi::Record,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    PartialEq,
    Default,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct Hl7MessageHeader {
    pub sending_app: String,
    pub sending_facility: String,
    pub receiving_app: String,
    pub receiving_facility: String,
    pub timestamp: String,
    pub message_type: String,
    pub trigger_event: String,
    pub message_control_id: String,
    pub processing_id: String,
    pub version_id: String,
}

/// HL7 v2 Patient Identification (PID)
#[derive(
    uniffi::Record,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    PartialEq,
    Default,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct Hl7PatientInfo {
    pub patient_id: String,
    pub mrn: String,
    pub family_name: String,
    pub given_name: String,
    pub middle_name: String,
    pub dob: String,
    pub gender: String,
    pub address: String,
    pub phone: String,
}

/// HL7 v2 Patient Visit (PV1)
#[derive(
    uniffi::Record,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    PartialEq,
    Default,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct Hl7VisitInfo {
    pub patient_class: String,
    pub assigned_location: String,
    pub attending_doctor: String,
    pub visit_number: String,
    pub admit_date_time: String,
}

/// HL7 v2 Observation / Result (OBX)
#[derive(
    uniffi::Record,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    PartialEq,
    Default,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct Hl7Observation {
    pub set_id: String,
    pub value_type: String,
    pub observation_code: String,
    pub observation_text: String,
    pub value: String,
    pub units: String,
    pub reference_range: String,
    pub abnormal_flags: String,
    pub result_status: String,
}

/// HL7 v2 Order Information (ORC / OBR)
#[derive(
    uniffi::Record,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    PartialEq,
    Default,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct Hl7OrderInfo {
    pub order_control: String,
    pub placer_order_number: String,
    pub filler_order_number: String,
    pub order_code: String,
    pub order_text: String,
}

#[derive(
    uniffi::Record,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    PartialEq,
    Default,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct Hl7ZSegment {
    pub segment_id: String,
    pub raw_fields: Vec<String>,
}

/// Parsed HL7 v2 Message Payload Structure
#[derive(
    uniffi::Record,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    PartialEq,
    Default,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct Hl7ParsedPayload {
    pub header: Hl7MessageHeader,
    pub patient: Option<Hl7PatientInfo>,
    pub visit: Option<Hl7VisitInfo>,
    pub observations: Vec<Hl7Observation>,
    pub orders: Vec<Hl7OrderInfo>,
    pub z_segments: Vec<Hl7ZSegment>,
    pub raw_er7: String,
}

/// Helper function to split field components by '^'
fn get_component<'a>(field: &'a str, index: usize) -> &'a str {
    field.split('^').nth(index).unwrap_or("").trim()
}

/// Parse HL7 v2 ER7 pipe-delimited string into structured payload
pub fn parse_hl7_v2_message(raw_er7: &str) -> Result<Hl7ParsedPayload, String> {
    let normalized = raw_er7.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized
        .lines()
        .map(|l| l.trim_matches(|c| c == '\r' || c == '\n').trim_start())
        .filter(|l| !l.is_empty())
        .collect();

    if lines.is_empty() {
        return Err("Empty HL7 payload".to_string());
    }

    let mut header = Hl7MessageHeader::default();
    let mut patient: Option<Hl7PatientInfo> = None;
    let mut visit: Option<Hl7VisitInfo> = None;
    let mut observations = Vec::new();
    let mut orders = Vec::new();
    let mut z_segments = Vec::new();

    let mut current_order: Option<Hl7OrderInfo> = None;

    for line in lines {
        let fields: Vec<&str> = line.split('|').collect();
        if fields.is_empty() {
            continue;
        }

        let seg_id = fields[0];
        if seg_id.starts_with('Z') || seg_id.starts_with('z') {
            z_segments.push(Hl7ZSegment {
                segment_id: seg_id.to_string(),
                raw_fields: fields.iter().map(|s| s.to_string()).collect(),
            });
            continue;
        }

        match seg_id {
            "MSH" => {
                header.sending_app = fields.get(2).unwrap_or(&"").to_string();
                header.sending_facility = fields.get(3).unwrap_or(&"").to_string();
                header.receiving_app = fields.get(4).unwrap_or(&"").to_string();
                header.receiving_facility = fields.get(5).unwrap_or(&"").to_string();
                header.timestamp = fields.get(6).unwrap_or(&"").to_string();

                if let Some(msg_type_field) = fields.get(8) {
                    header.message_type = get_component(msg_type_field, 0).to_string();
                    header.trigger_event = get_component(msg_type_field, 1).to_string();
                }
                header.message_control_id = fields.get(9).unwrap_or(&"").to_string();
                header.processing_id = fields.get(10).unwrap_or(&"").to_string();
                header.version_id = fields.get(11).unwrap_or(&"").to_string();
            }
            "PID" => {
                let mut p = Hl7PatientInfo::default();
                p.patient_id = fields.get(2).unwrap_or(&"").to_string();
                
                let mrn_field = fields.get(3).unwrap_or(&"");
                p.mrn = if mrn_field.contains('^') {
                    get_component(mrn_field, 0).to_string()
                } else {
                    mrn_field.to_string()
                };
                if p.mrn.is_empty() {
                    p.mrn = p.patient_id.clone();
                }

                let name_field = fields.get(5).unwrap_or(&"");
                p.family_name = get_component(name_field, 0).to_string();
                p.given_name = get_component(name_field, 1).to_string();
                p.middle_name = get_component(name_field, 2).to_string();

                p.dob = fields.get(7).unwrap_or(&"").to_string();
                p.gender = fields.get(8).unwrap_or(&"").to_string();
                p.address = fields.get(11).unwrap_or(&"").to_string();
                p.phone = fields.get(13).unwrap_or(&"").to_string();
                patient = Some(p);
            }
            "PV1" => {
                let mut v = Hl7VisitInfo::default();
                v.patient_class = fields.get(2).unwrap_or(&"").to_string();
                v.assigned_location = fields.get(3).unwrap_or(&"").to_string();
                v.attending_doctor = fields.get(7).unwrap_or(&"").to_string();
                v.visit_number = get_component(fields.get(19).unwrap_or(&""), 0).to_string();
                v.admit_date_time = fields.get(44).unwrap_or(&"").to_string();
                visit = Some(v);
            }
            "ORC" => {
                let mut ord = Hl7OrderInfo::default();
                ord.order_control = fields.get(1).unwrap_or(&"").to_string();
                ord.placer_order_number = fields.get(2).unwrap_or(&"").to_string();
                ord.filler_order_number = fields.get(3).unwrap_or(&"").to_string();
                current_order = Some(ord);
            }
            "OBR" => {
                let mut ord = current_order.take().unwrap_or_default();
                if ord.placer_order_number.is_empty() {
                    ord.placer_order_number = fields.get(2).unwrap_or(&"").to_string();
                }
                if ord.filler_order_number.is_empty() {
                    ord.filler_order_number = fields.get(3).unwrap_or(&"").to_string();
                }
                let code_field = fields.get(4).unwrap_or(&"");
                ord.order_code = get_component(code_field, 0).to_string();
                ord.order_text = get_component(code_field, 1).to_string();
                orders.push(ord);
            }
            "OBX" => {
                let mut obs = Hl7Observation::default();
                obs.set_id = fields.get(1).unwrap_or(&"").to_string();
                obs.value_type = fields.get(2).unwrap_or(&"").to_string();

                let obs_id_field = fields.get(3).unwrap_or(&"");
                obs.observation_code = get_component(obs_id_field, 0).to_string();
                obs.observation_text = get_component(obs_id_field, 1).to_string();

                obs.value = fields.get(5).unwrap_or(&"").to_string();
                obs.units = fields.get(6).unwrap_or(&"").to_string();
                obs.reference_range = fields.get(7).unwrap_or(&"").to_string();
                obs.abnormal_flags = fields.get(8).unwrap_or(&"").to_string();
                obs.result_status = fields.get(11).unwrap_or(&"").to_string();
                observations.push(obs);
            }
            _ => {}
        }
    }

    if header.message_type.is_empty() {
        return Err("Missing valid MSH segment header in HL7 payload".to_string());
    }

    Ok(Hl7ParsedPayload {
        header,
        patient,
        visit,
        observations,
        orders,
        z_segments,
        raw_er7: raw_er7.to_string(),
    })
}

/// UniFFI helper function to extract a field value from a parsed Z-segment by segment ID and index.
#[uniffi::export]
pub fn get_z_segment_field(payload: &Hl7ParsedPayload, segment_id: &str, field_index: u32) -> Option<String> {
    for seg in &payload.z_segments {
        if seg.segment_id.eq_ignore_ascii_case(segment_id) {
            return seg.raw_fields.get(field_index as usize).cloned();
        }
    }
    None
}

/// Generate compliant HL7 v2 ACK (Acknowledgement) payload
pub fn generate_hl7_ack(header: &Hl7MessageHeader, ack_code: &str, text_message: &str) -> String {
    let now_ts = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    let ack_ctrl_id = format!("ACK_{}", header.message_control_id);
    let trigger = if header.trigger_event.is_empty() {
        "A01".to_string()
    } else {
        header.trigger_event.clone()
    };

    format!(
        "MSH|^~\\&|YNTRA_MLLP|YNTRA_HOSPITAL|{}|{}|{}||ACK^{}|{}|{}|{}\r\nMSA|{}|{}|{}\r\n",
        header.sending_app,
        header.sending_facility,
        now_ts,
        trigger,
        ack_ctrl_id,
        if header.processing_id.is_empty() { "P" } else { &header.processing_id },
        if header.version_id.is_empty() { "2.3" } else { &header.version_id },
        ack_code,
        header.message_control_id,
        text_message
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mllp_framing_encode_decode() {
        let sample_msg = "MSH|^~\\&|EPIC|HOSPITAL|YNTRA|CLINIC|20260806120000||ADT^A01|MSG99001|P|2.3\rPID|1||MRN123456||DOE^JOHN";
        let encoded = encode_mllp_frame(sample_msg);
        assert_eq!(encoded[0], MLLP_START_BLOCK);
        assert_eq!(encoded[encoded.len() - 2], MLLP_END_BLOCK_1);
        assert_eq!(encoded[encoded.len() - 1], MLLP_END_BLOCK_2);

        let mut buffer = Vec::new();
        buffer.extend_from_slice(&[0x00, 0x01]); // leading noise
        buffer.extend_from_slice(&encoded);
        buffer.extend_from_slice(&[0x02]); // trailing noise

        let frames = decode_mllp_frames(&mut buffer);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0], sample_msg);
    }

    #[test]
    fn test_parse_hl7_adt_a01() {
        let er7 = "MSH|^~\\&|EPIC_EHR|GENERAL_HOSP|YNTRA_CORE|YNTRA_FAC|20260806140000||ADT^A01|CTRL1001|P|2.3\r\
                   PID|1||MRN778899^^^HOSP||SMITH^JANE^ELIZABETH||19920515|F|||456 PARK AVE^^SAN FRANCISCO^CA^94102||555-0199||\r\
                   PV1|1|I|ICU^BED-04^ROOM-101||||1234^STEVENS^MARK||||||||||||V998877|||||||||||||||||||||||||20260806140000|";

        let parsed = parse_hl7_v2_message(er7).expect("Failed to parse ADT^A01");
        assert_eq!(parsed.header.sending_app, "EPIC_EHR");
        assert_eq!(parsed.header.message_type, "ADT");
        assert_eq!(parsed.header.trigger_event, "A01");
        assert_eq!(parsed.header.message_control_id, "CTRL1001");

        let patient = parsed.patient.expect("Missing patient");
        assert_eq!(patient.mrn, "MRN778899");
        assert_eq!(patient.family_name, "SMITH");
        assert_eq!(patient.given_name, "JANE");
        assert_eq!(patient.gender, "F");

        let visit = parsed.visit.expect("Missing visit");
        assert_eq!(visit.patient_class, "I");
        assert_eq!(visit.assigned_location, "ICU^BED-04^ROOM-101");
        assert_eq!(visit.visit_number, "V998877");
    }

    #[test]
    fn test_parse_hl7_oru_r01() {
        let er7 = "MSH|^~\\&|LAB_SYS|CENTRAL_LAB|YNTRA|YNTRA_FAC|20260806150000||ORU^R01|CTRL2002|P|2.3\r\
                   PID|1||MRN112233||BROWN^ROBERT||19751020|M\r\
                   OBR|1|ORD5544|LAB8877|CBC^COMPLETE BLOOD COUNT\r\
                   OBX|1|NM|WBC^WHITE BLOOD CELL COUNT||7.5|10^3/uL|4.5-11.0|N|||F\r\
                   OBX|2|NM|RBC^RED BLOOD CELL COUNT||4.8|10^6/uL|4.2-5.4|N|||F";

        let parsed = parse_hl7_v2_message(er7).expect("Failed to parse ORU^R01");
        assert_eq!(parsed.header.message_type, "ORU");
        assert_eq!(parsed.observations.len(), 2);
        assert_eq!(parsed.observations[0].observation_code, "WBC");
        assert_eq!(parsed.observations[0].value, "7.5");
        assert_eq!(parsed.observations[1].observation_code, "RBC");
        assert_eq!(parsed.observations[1].value, "4.8");
    }

    #[test]
    fn test_generate_ack() {
        let header = Hl7MessageHeader {
            sending_app: "EPIC".to_string(),
            sending_facility: "HOSP".to_string(),
            receiving_app: "YNTRA".to_string(),
            receiving_facility: "CLINIC".to_string(),
            timestamp: "20260806120000".to_string(),
            message_type: "ADT".to_string(),
            trigger_event: "A01".to_string(),
            message_control_id: "CTRL999".to_string(),
            processing_id: "P".to_string(),
            version_id: "2.3".to_string(),
        };

        let ack = generate_hl7_ack(&header, "AA", "Message processed successfully");
        assert!(ack.contains("MSA|AA|CTRL999|Message processed successfully"));
        assert!(ack.contains("ACK^A01"));
    }

    #[test]
    fn test_dynamic_z_segment_passthrough() {
        let er7 = "MSH|^~\\&|EPIC_EHR|HOSP|YNTRA|FAC|20260806160000||ADT^A08|CTRL9009|P|2.3\r\
                   PID|1||MRN9900||DOE^JOHN||19800101|M\r\
                   ZPD|1|EPIC_PATIENT_PREF_A|CONFIDENTIAL_VIP_TRUE|ROOM_BED_OPT_B\r\
                   ZAL|1|PENICILLIN_SEVERE|SHELLFISH_MODERATE";

        let parsed = parse_hl7_v2_message(er7).expect("Failed to parse message with Z-segments");
        assert_eq!(parsed.z_segments.len(), 2);
        assert_eq!(parsed.z_segments[0].segment_id, "ZPD");
        assert_eq!(parsed.z_segments[1].segment_id, "ZAL");

        let vip_pref = get_z_segment_field(&parsed, "ZPD", 3).expect("Missing ZPD-3 field");
        assert_eq!(vip_pref, "CONFIDENTIAL_VIP_TRUE");

        let severe_allergy = get_z_segment_field(&parsed, "ZAL", 2).expect("Missing ZAL-2 field");
        assert_eq!(severe_allergy, "PENICILLIN_SEVERE");
    }
}
