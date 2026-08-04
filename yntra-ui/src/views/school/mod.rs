use dioxus::prelude::*;

pub mod academics;
pub mod attendance;
pub mod finance;
pub mod health;
pub mod library;
pub mod report_cards;
pub mod student_directory;

pub use academics::AcademicsView;
pub use attendance::AttendanceView;
pub use finance::FinanceView;
pub use health::HealthClinicView;
pub use library::LibraryView;
pub use report_cards::ReportCardsView;
pub use student_directory::StudentDirectoryView;

#[derive(Props, Clone, PartialEq)]
pub struct SchoolViewProps {
    pub active_user_id: String,
    pub workspace_id: String,
    pub block_id: String,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

pub fn infer_subject_from_course(course: &str, locale: &str) -> Option<String> {
    let name_lower = course.trim().to_lowercase();
    if name_lower.is_empty() {
        return None;
    }

    // Mathematics
    let math_keywords = &[
        "algebra",
        "geometry",
        "calculus",
        "geometri",
        "analys",
        "kalkulus",
        "geometria",
        "analyysi",
    ];
    if math_keywords.iter().any(|k| name_lower.contains(k)) {
        return Some(match locale {
            "sv" => "Matematik".to_string(),
            "no" => "Matematikk".to_string(),
            "da" => "Matematik".to_string(),
            "fi" => "Matematiikka".to_string(),
            _ => "Mathematics".to_string(),
        });
    }

    // Science
    let science_keywords = &[
        "biology",
        "physics",
        "chemistry",
        "biologi",
        "fysik",
        "kemi",
        "science",
        "naturvetenskap",
        "naturfag",
        "luonnontiede",
        "fysiikka",
        "kemia",
        "biologia",
        "fysikk",
        "kjemi",
    ];
    if science_keywords.iter().any(|k| name_lower.contains(k)) {
        return Some(match locale {
            "sv" => "Naturvetenskap".to_string(),
            "no" => "Naturfag".to_string(),
            "da" => "Naturfag".to_string(),
            "fi" => "Luonnontiede".to_string(),
            _ => "Science".to_string(),
        });
    }

    // History & Geography
    let history_keywords = &[
        "history",
        "geography",
        "civilization",
        "historia",
        "geografi",
        "historie",
        "civilisation",
        "civilisasjon",
        "sivilisaatio",
        "maantieto",
    ];
    if history_keywords.iter().any(|k| name_lower.contains(k)) {
        return Some(match locale {
            "sv" => "Historia".to_string(),
            "no" => "Historie".to_string(),
            "da" => "Historie".to_string(),
            "fi" => "Historia".to_string(),
            _ => "History".to_string(),
        });
    }

    // English
    let english_keywords = &[
        "english",
        "literature",
        "writing",
        "engelska",
        "engelsk",
        "englanti",
        "kirjallisuus",
        "skrivande",
        "skriving",
    ];
    if english_keywords.iter().any(|k| name_lower.contains(k)) {
        return Some(match locale {
            "sv" => "Engelska".to_string(),
            "no" => "Engelsk".to_string(),
            "da" => "Engelsk".to_string(),
            "fi" => "Englanti".to_string(),
            _ => "English".to_string(),
        });
    }

    // Art
    let art_keywords = &[
        "drawing",
        "painting",
        "art",
        "ceramics",
        "sketching",
        "design",
        "bild",
        "kuvataide",
        "billedkunst",
        "kunst",
    ];
    if art_keywords.iter().any(|k| name_lower.contains(k)) {
        return Some(match locale {
            "sv" => "Bild".to_string(),
            "no" => "Kunst og håndverk".to_string(),
            "da" => "Billedkunst".to_string(),
            "fi" => "Kuvataide".to_string(),
            _ => "Art".to_string(),
        });
    }

    // Music
    let music_keywords = &[
        "music",
        "choir",
        "band",
        "orchestra",
        "guitar",
        "musik",
        "musiikki",
        "kor",
        "orkester",
    ];
    if music_keywords.iter().any(|k| name_lower.contains(k)) {
        return Some(match locale {
            "sv" => "Musik".to_string(),
            "no" => "Musikk".to_string(),
            "da" => "Musik".to_string(),
            "fi" => "Musiikki".to_string(),
            _ => "Music".to_string(),
        });
    }

    // PE
    let pe_keywords = &[
        "fitness",
        "conditioning",
        "sports",
        "wellness",
        "yoga",
        "gymnastics",
        "idrott",
        "hälsa",
        "kroppsøving",
        "idræt",
        "liikunta",
        "terveystieto",
    ];
    if pe_keywords.iter().any(|k| name_lower.contains(k)) {
        return Some(match locale {
            "sv" => "Idrott och hälsa".to_string(),
            "no" => "Kroppsøving".to_string(),
            "da" => "Idræt".to_string(),
            "fi" => "Liikunta".to_string(),
            _ => "Physical Education".to_string(),
        });
    }

    None
}
