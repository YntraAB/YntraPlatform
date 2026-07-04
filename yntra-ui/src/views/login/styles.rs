pub fn get_keyframes_css(brand_color: &str) -> String {
    format!(
        r#"
        @keyframes scan-laser {{
            0% {{ top: 0%; }}
            50% {{ top: 100%; }}
            100% {{ top: 0%; }}
        }}
        @keyframes sonar-wave {{
            0% {{
                transform: scale(0.6);
                opacity: 1;
            }}
            100% {{
                transform: scale(1.4);
                opacity: 0;
            }}
        }}
        .login-primary-btn {{
            background: {brand_color} !important;
            color: white !important;
            border: none !important;
            width: 100% !important;
        }}
        .login-primary-btn:hover {{
            filter: brightness(0.9) !important;
        }}
        .google-btn {{
            background: hsl(217.2, 91.2%, 59.8%) !important;
            color: white !important;
            border: none !important;
            width: 100% !important;
            gap: 0.75rem !important;
        }}
        .google-btn:hover {{
            background: hsl(217.2, 91.2%, 54.8%) !important;
            opacity: 0.95;
        }}
        .github-btn {{
            background: hsl(240, 10%, 3.9%) !important;
            color: white !important;
            border: 1px solid var(--border-color) !important;
            width: 100% !important;
            gap: 0.75rem !important;
        }}
        .github-btn:hover {{
            background: hsl(240, 5.9%, 10%) !important;
            border-color: rgba(99, 102, 241, 0.45) !important;
        }}
        .login-card .yntra-btn {{
            width: 100% !important;
            box-sizing: border-box !important;
        }}
        "#
    )
}
