//! Oberfläche für Import/Export der Stammdaten:
//! - `ueberschrift()`: Seitentitel mit den Knöpfen "Importieren" / "Exportieren"
//!   (auf den Seiten Feuerwehr, Fahrzeugpark, Personal, Sprechgruppen)
//! - `dialog()`: Vorschau mit Konfliktprüfung vor dem eigentlichen Import
//!
//! Die eigentliche Logik (JSON, Vergleich, Datenbank) liegt in `stammdaten.rs`.

use crate::stammdaten::{self, Analyse, Art, DuplikatModus, Optionen, Paket, Status};
use crate::{theme, App};
use eframe::egui::{self, Align, Color32, Layout, RichText};

/// Geladene Import-Datei samt Prüfergebnis, solange der Vorschau-Dialog offen ist.
pub struct ImportVorschau {
    /// Pfad der Datei (nur zur Anzeige).
    pub datei: String,
    pub paket: Paket,
    pub analyse: Analyse,
    pub optionen: Optionen,
    /// Detailliste auf Konflikte/Duplikate/Fehler beschränken.
    pub nur_probleme: bool,
}

/// Seitentitel links, Import-/Export-Knöpfe rechts.
pub fn ueberschrift(app: &mut App, ui: &mut egui::Ui, titel: &str) {
    ui.horizontal(|ui| {
        ui.heading(titel);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .button("⬇ Exportieren")
                .on_hover_text(
                    "Alle Stammdaten (Feuerwehren, Fahrzeuge, Personal, Qualifikationen, Sprechgruppen) in eine JSON-Datei speichern",
                )
                .clicked()
            {
                exportieren(app);
            }
            if ui
                .button("⬆ Importieren")
                .on_hover_text("Stammdaten aus einer JSON-Datei übernehmen (mit Konfliktprüfung)")
                .clicked()
            {
                import_vorbereiten(app);
            }
        });
    });
}

/// Fragt nach dem Speicherort und schreibt alle Stammdaten als JSON.
fn exportieren(app: &mut App) {
    let paket = match stammdaten::exportieren(app.db.verbindung()) {
        Ok(p) => p,
        Err(e) => {
            app.fehler(format!("Export fehlgeschlagen: {e}"));
            return;
        }
    };
    let vorschlag = format!("firecage_stammdaten_{}.json", chrono::Local::now().format("%Y-%m-%d"));
    let Some(pfad) = rfd::FileDialog::new()
        .set_title("Stammdaten exportieren")
        .add_filter("Firecage-Stammdaten (JSON)", &["json"])
        .set_file_name(vorschlag)
        .save_file()
    else {
        return; // abgebrochen
    };
    match stammdaten::schreiben(&paket, &pfad) {
        Ok(()) => app.info(format!("Exportiert nach {}: {}", pfad.display(), paket.zusammenfassung())),
        Err(e) => app.fehler(e),
    }
}

/// Fragt nach der Datei, liest sie ein, prüft auf Konflikte und öffnet die Vorschau.
fn import_vorbereiten(app: &mut App) {
    let Some(pfad) = rfd::FileDialog::new()
        .set_title("Stammdaten importieren")
        .add_filter("Firecage-Stammdaten (JSON)", &["json"])
        .pick_file()
    else {
        return; // abgebrochen
    };
    let paket = match stammdaten::lesen(&pfad) {
        Ok(p) => p,
        Err(e) => {
            app.fehler(e);
            return;
        }
    };
    let analyse = match stammdaten::analysieren(app.db.verbindung(), &paket) {
        Ok(a) => a,
        Err(e) => {
            app.fehler(format!("Prüfung fehlgeschlagen: {e}"));
            return;
        }
    };
    app.hinweis = None;
    app.f.import = Some(ImportVorschau {
        datei: pfad.display().to_string(),
        paket,
        analyse,
        optionen: Optionen::default(),
        nur_probleme: true,
    });
}

/// Farbe je Prüfstatus.
fn status_farbe(s: Status) -> Color32 {
    match s {
        Status::Neu => theme::GRUEN,
        Status::Gleich => theme::TEXT_HELL,
        Status::Konflikt | Status::Duplikat => theme::ORANGE,
        Status::Fehler => theme::ROT,
    }
}

/// Vorschau-Dialog vor dem Import (nur sichtbar, wenn eine Datei geladen ist).
/// Zeigt eine Übersicht je Datensatzart, die Optionen für Konflikte/Duplikate
/// und eine Detailliste. Erst "Importieren" schreibt in die Datenbank.
pub fn dialog(app: &mut App, ctx: &egui::Context) {
    let Some(v) = app.f.import.as_mut() else { return };

    // Abgedunkelter Hintergrund, blockiert Klicks auf die restliche Oberfläche
    egui::Area::new(egui::Id::new("import_hintergrund"))
        .order(egui::Order::Middle)
        .fixed_pos(egui::Pos2::ZERO)
        .show(ctx, |ui| {
            let flaeche = ctx.screen_rect();
            ui.allocate_rect(flaeche, egui::Sense::click());
            ui.painter().rect_filled(flaeche, 0.0, Color32::from_black_alpha(170));
        });

    let mut abbrechen = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    let mut ausfuehren = false;

    egui::Area::new(egui::Id::new("import_dialog"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            egui::Frame::window(ui.style()).inner_margin(20.0).show(ui, |ui| {
                ui.set_width(950.0);
                ui.heading("Stammdaten importieren");
                ui.label(format!("Datei: {}", v.datei));
                ui.label(format!("Exportiert am {}  ·  {}", v.paket.exportiert_am, v.paket.zusammenfassung()));
                ui.add_space(10.0);

                // ── Übersicht: Art × Status ──
                egui::Grid::new("import_uebersicht")
                    .striped(true)
                    .spacing([24.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("");
                        for s in Status::ALLE {
                            ui.strong(s.name());
                        }
                        ui.end_row();
                        for art in Art::ALLE {
                            ui.label(art.name());
                            for s in Status::ALLE {
                                let n = v.analyse.anzahl(art, s);
                                let farbe = if n > 0 { status_farbe(s) } else { Color32::from_gray(110) };
                                ui.colored_label(farbe, n.to_string());
                            }
                            ui.end_row();
                        }
                    });
                ui.add_space(10.0);

                // ── Optionen ──
                let konflikte = v.analyse.summe(Status::Konflikt);
                let duplikate = v.analyse.summe(Status::Duplikat);
                let fehler = v.analyse.summe(Status::Fehler);
                if konflikte > 0 {
                    ui.label(RichText::new(format!("{konflikte} Konflikt(e) – gleiche Kennung, aber unterschiedliche Daten:")).strong());
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut v.optionen.konflikt_ueberschreiben, false, "lokale Daten behalten");
                        ui.radio_value(&mut v.optionen.konflikt_ueberschreiben, true, "Daten aus der Datei übernehmen");
                    });
                }
                if duplikate > 0 {
                    ui.label(RichText::new(format!("{duplikate} mögliche(s) Duplikat(e) – gleicher Name, andere Kennung:")).strong());
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut v.optionen.duplikat, DuplikatModus::Ueberspringen, "überspringen");
                        ui.radio_value(
                            &mut v.optionen.duplikat,
                            DuplikatModus::Zusammenfuehren,
                            "lokalen Eintrag aktualisieren",
                        );
                        ui.radio_value(&mut v.optionen.duplikat, DuplikatModus::NeuAnlegen, "zusätzlich neu anlegen");
                    });
                }
                if fehler > 0 {
                    ui.colored_label(
                        theme::ROT,
                        format!("{fehler} Eintrag/Einträge können nicht importiert werden und werden übersprungen."),
                    );
                }
                if konflikte + duplikate + fehler == 0 {
                    ui.colored_label(theme::GRUEN, "Keine Konflikte gefunden.");
                }
                ui.add_space(10.0);

                // ── Detailliste ──
                ui.checkbox(&mut v.nur_probleme, "nur Konflikte, Duplikate und Fehler anzeigen");
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    egui::Grid::new("import_details")
                        .striped(true)
                        .num_columns(4)
                        .spacing([18.0, 4.0])
                        .show(ui, |ui| {
                            for kopf in ["Art", "Eintrag", "Status", "Details"] {
                                ui.strong(kopf);
                            }
                            ui.end_row();
                            let probleme = [Status::Konflikt, Status::Duplikat, Status::Fehler];
                            for e in v
                                .analyse
                                .eintraege
                                .iter()
                                .filter(|e| !v.nur_probleme || probleme.contains(&e.status))
                            {
                                ui.label(e.art.name());
                                ui.label(e.bezeichnung.as_str());
                                ui.colored_label(status_farbe(e.status), e.status.name());
                                ui.add(egui::Label::new(e.hinweis.as_str()).wrap());
                                ui.end_row();
                            }
                        });
                });

                // ── Knöpfe ──
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("✔ Importieren").strong()).clicked() {
                        ausfuehren = true;
                    }
                    if ui.button("Abbrechen").clicked() {
                        abbrechen = true;
                    }
                });
            });
        });

    if ausfuehren {
        if let Some(v) = app.f.import.take() {
            // Import läuft in einer Transaktion: bei einem Fehler bleibt alles unverändert.
            match stammdaten::importieren(app.db.verbindung(), &v.paket, &v.optionen) {
                Ok(b) => app.info(b.text()),
                Err(e) => app.fehler(format!("Import abgebrochen, es wurde nichts geändert: {e}")),
            }
        }
    } else if abbrechen {
        app.f.import = None;
    }
}
