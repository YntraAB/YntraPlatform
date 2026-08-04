---
title: "Industry Templates & Workspace Blueprints"
head:
  - tag: script
    attrs:
      src: /target-blank.js
sidebar:
  hidden: false
---

Yntra provides pre-built operational workspace blueprints tailored for specific industries.

---

## 1. Available Industry Blueprints

1. **Construction & Field Services**: Includes Job Tickets, Equipment Checklists, Daily Site Notes, and Geofenced Time Clock.
2. **Healthcare & Home Assistance**: Includes Patient Profiles, Care Logs, Medication Schedules, and Handover Notes.
3. **Education & School Administration**: Includes Student Roster, Attendance Tracking, Lending Logs, and Staff Messages.
4. **Vehicle & Fleet Management**: Includes Mileage Reporting, Inspection Photos, and Damage Log Pointers.

---

## 2. Instantiating a Template

Industry templates are selected during workspace setup ([`setup.rs`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-ui/src/views/setup.rs)) or via `services::industry_templates::apply_industry_template()`.