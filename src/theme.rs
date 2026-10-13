//! Farben, Schriftgrößen und wiederverwendbare Bedienelemente im Firecage-Design
//! (angelehnt an Aussehen/Zeichnung.png).

use eframe::egui::{self, Color32, FontId, RichText, Sense, Stroke, TextStyle};

/// Hintergrund der Kopfzeile.
pub const KOPF: Color32 = Color32::from_rgb(61, 61, 61);
/// Hintergrund des Hauptbereichs.
pub const HINTERGRUND: Color32 = Color32::from_rgb(61, 61, 61);
/// Hintergrund der Seitenleiste.
pub const SEITE: Color32 = Color32::from_rgb(106, 106, 106);
/// Hellgraue Menüknöpfe.
pub const KNOPF: Color32 = Color32::from_rgb(214, 214, 214);
/// Menüknopf der gerade geöffneten Ansicht.
pub const KNOPF_AKTIV: Color32 = Color32::from_rgb(245, 245, 245);
/// Heller Text auf dunklem Grund (Überschriften).
pub const TEXT_HELL: Color32 = Color32::from_rgb(200, 200, 200);
/// Dunkler Text auf hellem Grund (Knöpfe, Uhr).
pub const TEXT_DUNKEL: Color32 = Color32::from_rgb(40, 40, 40);
/// Rot für Einsatzdauer, Warnungen und Fehler.
pub const ROT: Color32 = Color32::from_rgb(240, 128, 128);
/// Orange für Vorwarnungen (z. B. Atemschutz-Druckkontrolle).
pub const ORANGE: Color32 = Color32::from_rgb(240, 170, 60);
/// Grün für Erfolgsmeldungen.
pub const GRUEN: Color32 = Color32::from_rgb(120, 200, 120);

/// Das Firecage-Logo, direkt in die Programmdatei eingebettet.
pub const LOGO: &[u8] = include_bytes!("../Firecage icon.svg");

/// Setzt Grundfarben und Schriftgrößen für die ganze Anwendung.
pub fn anwenden(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = HINTERGRUND;
    v.window_fill = Color32::from_rgb(70, 70, 70);
    // Trennlinien zwischen Kopfzeile / Seitenleiste / Hauptbereich (hellgrau wie in der Zeichnung).
    v.widgets.noninteractive.bg_stroke = Stroke::new(2.0, Color32::from_gray(170));
    v.widgets.inactive.rounding = egui::Rounding::same(5.0);
    v.widgets.hovered.rounding = egui::Rounding::same(5.0);
    v.widgets.active.rounding = egui::Rounding::same(5.0);
    ctx.set_visuals(v);

    ctx.style_mut(|s| {
        s.text_styles.insert(TextStyle::Body, FontId::proportional(16.0));
        s.text_styles.insert(TextStyle::Button, FontId::proportional(16.0));
        s.text_styles.insert(TextStyle::Heading, FontId::proportional(26.0));
        s.text_styles.insert(TextStyle::Small, FontId::proportional(12.0));
        s.spacing.item_spacing = egui::vec2(10.0, 8.0);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
    });
}

/// Hellgrauer Menüknopf der Seitenleiste. `aktiv` hebt die geöffnete Ansicht hervor.
pub fn menue_knopf(ui: &mut egui::Ui, text: &str, aktiv: bool) -> egui::Response {
    let knopf = egui::Button::new(RichText::new(text).size(17.0).strong().color(TEXT_DUNKEL))
        .fill(if aktiv { KNOPF_AKTIV } else { KNOPF })
        .rounding(5.0)
        .min_size(egui::vec2(ui.available_width(), 34.0));
    ui.add(knopf)
}

/// Runder "+"-Knopf unten rechts. Gibt `true` zurück, wenn er geklickt wurde.
pub fn plus_knopf(ctx: &egui::Context) -> bool {
    egui::Area::new(egui::Id::new("plus_knopf"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-40.0, -40.0))
        .show(ctx, |ui| {
            let (rect, antwort) = ui.allocate_exact_size(egui::vec2(110.0, 110.0), Sense::click());
            let farbe = if antwort.hovered() {
                Color32::from_gray(125)
            } else {
                Color32::from_gray(100)
            };
            let p = ui.painter();
            let m = rect.center();
            p.circle_filled(m, 55.0, farbe);
            let strich = Stroke::new(13.0, Color32::from_gray(215));
            p.line_segment([m - egui::vec2(32.0, 0.0), m + egui::vec2(32.0, 0.0)], strich);
            p.line_segment([m - egui::vec2(0.0, 32.0), m + egui::vec2(0.0, 32.0)], strich);
            antwort.on_hover_text("Neuen Einsatz anlegen").clicked()
        })
        .inner
}

/// Wandelt eine Farbangabe "#RRGGBB" aus der Datenbank in eine egui-Farbe um.
/// Ungültige oder fehlende Angaben ergeben Hellgrau.
pub fn hex_farbe(hex: Option<&str>) -> Color32 {
    let standard = TEXT_HELL;
    let Some(h) = hex.map(|h| h.trim_start_matches('#')) else {
        return standard;
    };
    if h.len() != 6 {
        return standard;
    }
    let teil = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    match (teil(0), teil(2), teil(4)) {
        (Some(r), Some(g), Some(b)) => Color32::from_rgb(r, g, b),
        _ => standard,
    }
}
