//! Atemschutzüberwachung (angelehnt an Aussehen/Atemschutzansicht.png).
//!
//! - **Trupp-Leiste** unten: ein Reiter je Trupp ("Trupp 1 | 10:10 min"),
//!   eingefärbt nach Warnstufe, plus "＋" für einen neuen Trupp (max. 5).
//! - **Einzelansicht**: Tabelle ATF/ATM/WTF × Namen/Anfangsdruck/Druckabfrage/Enddruck,
//!   Knöpfe "Einsatz starten", "Druckabfrage", "Trupp zurück", große Zeitanzeige.
//! - **Tabellenansicht**: alle laufenden/wartenden Trupps auf einen Blick.
//! - **Abgeschlossen**: eigene Seite mit allen zurückgekehrten Trupps.
//!
//! Alle Eingaben werden sofort in der Datenbank gespeichert.

use super::{optional, uhrzeit, EinsatzTab};
use crate::model::AtsEinsatz;
use crate::{db, theme, App};
use eframe::egui::{self, Color32, FontId, RichText, Sense, Stroke};

/// Höchstzahl gleichzeitig geführter Trupps (wartend + im Einsatz).
pub const MAX_TRUPPS: usize = 5;

/// Bezeichnung und Farbe der drei Rollen (wie in der Zeichnung).
const ROLLEN: [&str; 3] = ["ATF", "ATM", "WTF"];
const ROLLEN_FARBE: [Color32; 3] = [
    Color32::from_rgb(222, 130, 130), // ATF – rot
    Color32::from_rgb(214, 200, 100), // ATM – gelb
    Color32::from_rgb(100, 160, 220), // WTF – blau
];

/// Spaltenüberschriften der Einzelansicht. Index = Spalte im Eingabepuffer.
const SPALTEN: [&str; 4] = ["Namen", "Anfangsdruck", "Druckabfrage", "Enddruck"];

/// Knopffarben aus der Zeichnung.
const KNOPF_GRUEN: Color32 = Color32::from_rgb(100, 200, 140);
const KNOPF_TUERKIS: Color32 = Color32::from_rgb(100, 200, 200);
const KNOPF_GRAU: Color32 = Color32::from_rgb(120, 120, 120);

// ───────────────────────────── Hilfsfunktionen ─────────────────────────────

/// "Huber / Maier / Gruber" – leere Namen werden ausgelassen.
pub fn trupp_namen(namen: &[String; 3]) -> String {
    let n: Vec<&str> = namen.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if n.is_empty() {
        "(keine Namen)".into()
    } else {
        n.join(" / ")
    }
}

/// Drücke einer Art für alle drei Rollen: "300 / 290 / –".
/// `art`: 0 = Anfangsdruck, 1 = Druckabfrage, 2 = Enddruck.
fn druecke(t: &AtsEinsatz, art: usize) -> String {
    (0..3)
        .map(|r| t.druck[r][art].map(|d| d.to_string()).unwrap_or_else(|| "–".into()))
        .collect::<Vec<_>>()
        .join(" / ")
}

/// Sekunden seit "Trupp geht vor" (bis jetzt bzw. bis "Trupp zurück").
fn vergangen(t: &AtsEinsatz, jetzt: i64) -> i64 {
    match t.start {
        Some(s) => (t.ende.unwrap_or(jetzt) - s).max(0),
        None => 0,
    }
}

/// "mm:ss"
fn mmss(sek: i64) -> String {
    format!("{:02}:{:02}", sek / 60, sek % 60)
}

/// Hat jedes eingetragene Truppmitglied einen Druckabfrage-Wert?
fn druckabfrage_vollstaendig(t: &AtsEinsatz) -> bool {
    let besetzt: Vec<usize> = (0..3).filter(|&r| !t.namen[r].trim().is_empty()).collect();
    !besetzt.is_empty() && besetzt.iter().all(|&r| t.druck[r][1].is_some())
}

/// Warnstufe eines Trupps: (Textfarbe, Reiterfarbe, Statustext).
/// - wartet: grau
/// - bis 1/3 der Zeit: grün
/// - ab 1/3: orange "Druckabfrage durchführen" (grün, wenn die Druckabfrage erledigt ist)
/// - ab 2/3: rot "Rückzug einleiten"; ab 100 %: rot "Einsatzzeit überschritten"
fn warnstufe(t: &AtsEinsatz, jetzt: i64) -> (Color32, Color32, &'static str) {
    if t.ende.is_some() {
        return (theme::TEXT_HELL, Color32::from_gray(90), "zurück");
    }
    if t.start.is_none() {
        return (theme::TEXT_HELL, Color32::from_gray(105), "wartet – noch nicht vorgegangen");
    }
    let anteil = vergangen(t, jetzt) as f32 / t.einsatzzeit.max(1) as f32;
    if anteil >= 1.0 {
        (theme::ROT, Color32::from_rgb(170, 60, 60), "⚠ EINSATZZEIT ÜBERSCHRITTEN")
    } else if anteil >= 2.0 / 3.0 {
        (theme::ROT, Color32::from_rgb(165, 80, 80), "⚠ Rückzug einleiten")
    } else if anteil >= 1.0 / 3.0 && !druckabfrage_vollstaendig(t) {
        (theme::ORANGE, Color32::from_rgb(185, 125, 45), "Druckabfrage durchführen")
    } else {
        (theme::GRUEN, Color32::from_rgb(150, 140, 70), "im Einsatz")
    }
}

/// Großer farbiger Knopf wie in der Zeichnung.
fn farb_knopf(text: &str, farbe: Color32) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text).size(18.0).strong().color(theme::TEXT_DUNKEL))
        .fill(farbe)
        .rounding(8.0)
        .min_size(egui::vec2(230.0, 46.0))
}

/// Füllt den Eingabepuffer eines Trupps aus den gespeicherten Werten.
fn puffer_aus(t: &AtsEinsatz) -> [[String; 4]; 3] {
    let mut p: [[String; 4]; 3] = Default::default();
    for r in 0..3 {
        p[r][0] = t.namen[r].clone();
        for art in 0..3 {
            p[r][art + 1] = t.druck[r][art].map(|d| d.to_string()).unwrap_or_default();
        }
    }
    p
}

// ───────────────────────────── Trupp-Leiste unten ─────────────────────────────

/// Leiste am unteren Rand: ein Reiter je nicht zurückgekehrtem Trupp
/// ("Trupp 1 | 10:10 min", Farbe = Warnstufe) und "＋" für einen neuen Trupp.
/// Muss in `update()` VOR dem Hauptbereich aufgerufen werden (egui-Panel-Reihenfolge).
pub fn trupp_leiste(app: &mut App, ctx: &egui::Context, einsatz_id: i64) {
    let liste = app.ok(app.db.ats_einsaetze(einsatz_id)).unwrap_or_default();
    let einsatz_laeuft = app.db.einsatz(einsatz_id).ok().flatten().is_some_and(|e| e.ende.is_none());
    let jetzt = db::jetzt();

    egui::TopBottomPanel::bottom("trupp_leiste")
        .exact_height(52.0)
        .frame(egui::Frame::none().fill(theme::SEITE))
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                let mut offen = 0;
                // Trupp-Nummer = Position in der Gesamtliste (bleibt stabil)
                for (i, t) in liste.iter().enumerate().filter(|(_, t)| t.ende.is_none()) {
                    offen += 1;
                    let (_, hintergrund, status) = warnstufe(t, jetzt);
                    let text = format!("Trupp {} | {} min", i + 1, mmss(vergangen(t, jetzt)));
                    let gewaehlt = app.f.ats_gewaehlt == Some(t.id) && !app.f.ats_tabelle && app.einsatz_tab == EinsatzTab::Atemschutz;
                    let knopf = egui::Button::new(RichText::new(text).size(16.0).strong().color(Color32::WHITE))
                        .fill(hintergrund)
                        .rounding(0.0)
                        .min_size(egui::vec2(190.0, 50.0))
                        .stroke(if gewaehlt { Stroke::new(3.0, Color32::WHITE) } else { Stroke::NONE });
                    if ui
                        .add(knopf)
                        .on_hover_text(format!("{}\n{}", trupp_namen(&t.namen), status))
                        .clicked()
                    {
                        app.f.ats_gewaehlt = Some(t.id);
                        app.f.ats_tabelle = false;
                        app.einsatz_tab = EinsatzTab::Atemschutz;
                    }
                }

                ui.add_space(10.0);
                if !einsatz_laeuft {
                    return;
                }
                if offen < MAX_TRUPPS {
                    if rund_plus(ui).on_hover_text("Neuen Atemschutztrupp anlegen").clicked() {
                        if let Some(id) = app.ok(app.db.ats_neu(einsatz_id)) {
                            app.f.ats_gewaehlt = Some(id);
                            app.f.ats_tabelle = false;
                            app.einsatz_tab = EinsatzTab::Atemschutz;
                        }
                    }
                } else {
                    ui.label(RichText::new(format!("max. {MAX_TRUPPS} Trupps")).color(theme::TEXT_HELL));
                }
            });
        });
}

/// Kleiner runder "+"-Knopf für die Trupp-Leiste.
fn rund_plus(ui: &mut egui::Ui) -> egui::Response {
    let (rect, antwort) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::click());
    let m = rect.center();
    let p = ui.painter();
    p.circle_filled(
        m,
        19.0,
        if antwort.hovered() {
            Color32::from_gray(95)
        } else {
            Color32::from_gray(70)
        },
    );
    let strich = Stroke::new(4.0, Color32::from_gray(225));
    p.line_segment([m - egui::vec2(10.0, 0.0), m + egui::vec2(10.0, 0.0)], strich);
    p.line_segment([m - egui::vec2(0.0, 10.0), m + egui::vec2(0.0, 10.0)], strich);
    antwort
}

// ───────────────────────────── Seite "Atemschutz" ─────────────────────────────

/// Seite "Atemschutz": Umschalter Einzel-/Tabellenansicht und der Inhalt.
pub fn zeigen(app: &mut App, ui: &mut egui::Ui, einsatz_id: i64) {
    let liste = app.ok(app.db.ats_einsaetze(einsatz_id)).unwrap_or_default();
    let jetzt = db::jetzt();

    ui.horizontal(|ui| {
        ui.selectable_value(&mut app.f.ats_tabelle, false, RichText::new("Einzelansicht").size(18.0));
        ui.selectable_value(&mut app.f.ats_tabelle, true, RichText::new("Tabellenansicht").size(18.0));
    });
    ui.add_space(10.0);

    if app.f.ats_tabelle {
        tabellenansicht(app, ui, &liste, jetzt);
        return;
    }

    // Gewählten Trupp prüfen; falls keiner (mehr) da: ersten offenen nehmen.
    let gueltig = app.f.ats_gewaehlt.is_some_and(|g| liste.iter().any(|t| t.id == g));
    if !gueltig {
        app.f.ats_gewaehlt = liste.iter().find(|t| t.ende.is_none()).map(|t| t.id);
    }
    match app.f.ats_gewaehlt.and_then(|g| liste.iter().position(|t| t.id == g)) {
        Some(i) => einzelansicht(app, ui, &liste[i], i + 1, jetzt),
        None => {
            ui.add_space(ui.available_height() * 0.35);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("Kein Atemschutztrupp im Einsatz")
                        .size(30.0)
                        .strong()
                        .color(theme::TEXT_HELL),
                );
                ui.label(
                    RichText::new("Mit ＋ in der Leiste unten einen Trupp anlegen.")
                        .size(18.0)
                        .color(theme::TEXT_HELL),
                );
            });
        }
    }
}

/// Was in der Einzelansicht geklickt wurde (wird nach dem Zeichnen ausgeführt).
enum Aktion {
    Keine,
    Starten,
    Druckabfrage,
    Zurueck,
    Verwerfen,
}

/// Einzelansicht eines Trupps: Tabelle, Knöpfe und Einsatzzeit.
fn einzelansicht(app: &mut App, ui: &mut egui::Ui, t: &AtsEinsatz, nr: usize, jetzt: i64) {
    let gestartet = t.start.is_some();
    let zurueck = t.ende.is_some();
    let (farbe, _, status) = warnstufe(t, jetzt);

    // Überschrift
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("ATS Trupp : Trupp {nr}"))
                .size(30.0)
                .strong()
                .color(theme::TEXT_HELL),
        );
        ui.add_space(20.0);
        ui.label(RichText::new(status).size(18.0).color(farbe));
    });
    ui.add_space(12.0);

    // ── Tabelle ATF/ATM/WTF ──
    // Eingaben landen im Puffer; geänderte Zellen werden danach gespeichert.
    let mut aenderungen: Vec<(usize, usize, String)> = Vec::new();
    let mut fokus_abfrage = std::mem::take(&mut app.f.ats_fokus_abfrage);
    let puffer = app.f.ats_puffer.entry(t.id).or_insert_with(|| puffer_aus(t));
    egui::Grid::new(("ats_tabelle", t.id))
        .num_columns(5)
        .spacing([28.0, 16.0])
        .show(ui, |ui| {
            ui.label("");
            for kopf in SPALTEN {
                ui.label(RichText::new(kopf).size(24.0).strong().color(theme::TEXT_HELL));
            }
            ui.end_row();

            for rolle in 0..3 {
                ui.label(RichText::new(ROLLEN[rolle]).size(30.0).strong().color(ROLLEN_FARBE[rolle]));
                for spalte in 0..4 {
                    // Wann darf welche Spalte bearbeitet werden?
                    let bearbeitbar = match spalte {
                        0 | 1 => !zurueck,          // Namen, Anfangsdruck: bis "Trupp zurück"
                        2 => gestartet && !zurueck, // Druckabfrage: nur während des Einsatzes
                        _ => gestartet,             // Enddruck: ab Start, auch nachträglich
                    };
                    let feld = egui::TextEdit::singleline(&mut puffer[rolle][spalte])
                        .font(FontId::proportional(20.0))
                        .desired_width(if spalte == 0 { 320.0 } else { 140.0 })
                        .hint_text(if spalte == 0 { "Name" } else { "bar" });
                    let antwort = ui.add_enabled(bearbeitbar, feld);
                    if spalte == 2 && rolle == 0 && fokus_abfrage && bearbeitbar {
                        antwort.request_focus();
                        fokus_abfrage = false;
                    }
                    if antwort.changed() {
                        aenderungen.push((rolle, spalte, puffer[rolle][spalte].clone()));
                    }
                }
                ui.end_row();
            }
        });
    speichern(app, t.id, aenderungen);

    // Max. Einsatzzeit (nur vor dem Start änderbar)
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Max. Einsatzzeit").size(18.0));
        let text = app
            .f
            .ats_zeit_puffer
            .entry(t.id)
            .or_insert_with(|| (t.einsatzzeit / 60).to_string());
        let antwort = ui.add_enabled(!gestartet, egui::TextEdit::singleline(&mut *text).desired_width(50.0));
        ui.label(RichText::new("min").size(18.0));
        if antwort.changed() {
            match text.trim().parse::<i64>() {
                Ok(min) if min > 0 => {
                    let r = app.db.ats_einsatzzeit_setzen(t.id, min * 60);
                    app.ok(r);
                }
                _ => app.fehler("Max. Einsatzzeit bitte als ganze Minuten (größer 0) angeben."),
            }
        }
    });

    // ── Knöpfe ──
    ui.add_space(16.0);
    let mut aktion = Aktion::Keine;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            if ui.add_enabled(!gestartet, farb_knopf("Einsatz starten", KNOPF_GRUEN)).clicked() {
                aktion = Aktion::Starten;
            }
            if ui
                .add_enabled(gestartet && !zurueck, farb_knopf("Trupp zurück", KNOPF_TUERKIS))
                .clicked()
            {
                aktion = Aktion::Zurueck;
            }
        });
        ui.add_space(30.0);
        // "Druckabfrage" setzt den Cursor in die Druckabfrage-Spalte;
        // das Kästchen daneben zeigt, ob für alle Truppmitglieder ein Wert eingetragen ist.
        if ui
            .add_enabled(gestartet && !zurueck, farb_knopf("Druckabfrage", KNOPF_GRAU))
            .clicked()
        {
            aktion = Aktion::Druckabfrage;
        }
        let (zeichen, zeichen_farbe, hilfe) = if druckabfrage_vollstaendig(t) {
            ("✔", theme::GRUEN, "Druckabfrage für alle Truppmitglieder eingetragen")
        } else {
            ("✖", theme::TEXT_HELL, "Druckabfrage noch nicht vollständig")
        };
        egui::Frame::none()
            .stroke(Stroke::new(1.5, theme::TEXT_HELL))
            .inner_margin(4.0)
            .show(ui, |ui| {
                ui.label(RichText::new(zeichen).size(24.0).color(zeichen_farbe));
            })
            .response
            .on_hover_text(hilfe);

        if !gestartet {
            ui.add_space(30.0);
            if ui
                .button("✖ Trupp verwerfen")
                .on_hover_text("Nur möglich, solange der Trupp noch nicht vorgegangen ist")
                .clicked()
            {
                aktion = Aktion::Verwerfen;
            }
        }
    });

    // ── Einsatzzeit ──
    ui.add_space(20.0);
    let sek = vergangen(t, jetzt);
    ui.label(
        RichText::new(format!("Einsatzzeit ATS Trupp: {} min", mmss(sek)))
            .size(32.0)
            .strong()
            .color(farbe),
    );
    if gestartet {
        let anteil = sek as f32 / t.einsatzzeit.max(1) as f32;
        ui.add(
            egui::ProgressBar::new(anteil.min(1.0))
                .desired_width(600.0)
                .fill(farbe)
                .text(format!(
                    "{} / {} min   (vorgegangen {})",
                    mmss(sek),
                    t.einsatzzeit / 60,
                    uhrzeit(t.start.unwrap_or(jetzt))
                )),
        );
    }

    // ── Aktion ausführen ──
    match aktion {
        Aktion::Keine => {}
        Aktion::Druckabfrage => app.f.ats_fokus_abfrage = true,
        Aktion::Starten => {
            // Mindestbesetzung: ATF und ATM mit Namen und Anfangsdruck
            let ok = (0..2).all(|r| !t.namen[r].trim().is_empty() && t.druck[r][0].is_some());
            if ok {
                let r = app.db.ats_starten(t.id);
                app.ok(r);
            } else {
                app.fehler("Zum Starten bitte für ATF und ATM Namen und Anfangsdruck eintragen.");
            }
        }
        Aktion::Zurueck => {
            let r = app.db.ats_beenden(t.id);
            if app.ok(r).is_some() {
                app.info(format!(
                    "Trupp {nr} ist zurück. Bitte den Enddruck eintragen, falls noch nicht geschehen."
                ));
            }
        }
        Aktion::Verwerfen => {
            let r = app.db.ats_verwerfen(t.id);
            if app.ok(r).is_some() {
                app.f.ats_puffer.remove(&t.id);
                app.f.ats_zeit_puffer.remove(&t.id);
                app.f.ats_gewaehlt = None;
            }
        }
    }
}

/// Speichert geänderte Tabellenzellen. Spalte 0 = Name, 1..3 = Drücke.
/// Ungültige Drücke werden nicht gespeichert, sondern als Fehler gemeldet.
fn speichern(app: &mut App, id: i64, aenderungen: Vec<(usize, usize, String)>) {
    for (rolle, spalte, text) in aenderungen {
        if spalte == 0 {
            let r = app.db.ats_name_setzen(id, rolle, &text);
            app.ok(r);
            continue;
        }
        let wert = match optional(&text) {
            None => None,
            Some(t) => match t.parse::<i64>() {
                Ok(bar) if (0..=400).contains(&bar) => Some(bar),
                _ => {
                    app.fehler(format!(
                        "{} {}: bitte ganze Zahl in bar (0–400) eingeben.",
                        ROLLEN[rolle], SPALTEN[spalte]
                    ));
                    continue;
                }
            },
        };
        let r = app.db.ats_druck_setzen(id, rolle, spalte, wert);
        app.ok(r);
    }
}

// ───────────────────────────── Tabellenansicht ─────────────────────────────

/// Alle nicht zurückgekehrten Trupps in einer Tabelle. "Öffnen" springt in die Einzelansicht.
fn tabellenansicht(app: &mut App, ui: &mut egui::Ui, liste: &[AtsEinsatz], jetzt: i64) {
    let offen: Vec<(usize, &AtsEinsatz)> = liste.iter().enumerate().filter(|(_, t)| t.ende.is_none()).collect();
    if offen.is_empty() {
        ui.label("Kein Atemschutztrupp im Einsatz.");
        return;
    }
    egui::ScrollArea::both().show(ui, |ui| {
        egui::Grid::new("ats_uebersicht")
            .striped(true)
            .num_columns(10)
            .spacing([22.0, 10.0])
            .show(ui, |ui| {
                for kopf in [
                    "Trupp",
                    "Status",
                    "ATF",
                    "ATM",
                    "WTF",
                    "Anfangsdruck",
                    "Druckabfrage",
                    "Vorgegangen",
                    "Einsatzzeit",
                    "",
                ] {
                    ui.strong(kopf);
                }
                ui.end_row();
                for (i, t) in offen {
                    let (farbe, _, status) = warnstufe(t, jetzt);
                    ui.label(RichText::new(format!("Trupp {}", i + 1)).strong());
                    ui.colored_label(farbe, status);
                    for r in 0..3 {
                        let name = if t.namen[r].is_empty() { "–" } else { t.namen[r].as_str() };
                        ui.colored_label(ROLLEN_FARBE[r], name);
                    }
                    ui.label(druecke(t, 0));
                    ui.label(druecke(t, 1));
                    ui.label(t.start.map(uhrzeit).unwrap_or_else(|| "–".into()));
                    ui.colored_label(farbe, format!("{} / {} min", mmss(vergangen(t, jetzt)), t.einsatzzeit / 60));
                    if ui.button("Öffnen").clicked() {
                        app.f.ats_gewaehlt = Some(t.id);
                        app.f.ats_tabelle = false;
                    }
                    ui.end_row();
                }
            });
    });
}

// ───────────────────────────── Seite "Atemschutz Abgeschlossen" ─────────────────────────────

/// Alle zurückgekehrten Trupps mit Drücken und Zeiten (Dokumentation).
pub fn abgeschlossen(app: &mut App, ui: &mut egui::Ui, einsatz_id: i64) {
    ui.label(
        RichText::new("Atemschutz – abgeschlossene Trupps")
            .size(26.0)
            .strong()
            .color(theme::TEXT_HELL),
    );
    ui.add_space(10.0);
    let liste = app.ok(app.db.ats_einsaetze(einsatz_id)).unwrap_or_default();
    let fertig: Vec<(usize, &AtsEinsatz)> = liste.iter().enumerate().filter(|(_, t)| t.ende.is_some()).collect();
    if fertig.is_empty() {
        ui.label("Noch kein Trupp zurückgekehrt.");
        return;
    }
    egui::ScrollArea::both().show(ui, |ui| {
        egui::Grid::new("ats_fertig")
            .striped(true)
            .num_columns(11)
            .spacing([22.0, 10.0])
            .show(ui, |ui| {
                for kopf in [
                    "Trupp",
                    "ATF",
                    "ATM",
                    "WTF",
                    "Anfangsdruck",
                    "Druckabfrage",
                    "Enddruck",
                    "Vorgegangen",
                    "Zurück",
                    "Dauer",
                    "",
                ] {
                    ui.strong(kopf);
                }
                ui.end_row();
                for (i, t) in fertig {
                    ui.label(RichText::new(format!("Trupp {}", i + 1)).strong());
                    for r in 0..3 {
                        let name = if t.namen[r].is_empty() { "–" } else { t.namen[r].as_str() };
                        ui.colored_label(ROLLEN_FARBE[r], name);
                    }
                    ui.label(druecke(t, 0));
                    ui.label(druecke(t, 1));
                    ui.label(druecke(t, 2));
                    ui.label(t.start.map(uhrzeit).unwrap_or_else(|| "–".into()));
                    ui.label(t.ende.map(uhrzeit).unwrap_or_else(|| "–".into()));
                    ui.label(format!("{} min", mmss(vergangen(t, t.ende.unwrap_or(0)))));
                    // "Details" öffnet die Einzelansicht, z. B. um den Enddruck nachzutragen.
                    if ui.button("Details").clicked() {
                        app.f.ats_gewaehlt = Some(t.id);
                        app.f.ats_tabelle = false;
                        app.einsatz_tab = EinsatzTab::Atemschutz;
                    }
                    ui.end_row();
                }
            });
    });
}
