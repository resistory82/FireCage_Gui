//! Startbildschirm ("Neuen Einsatz anlegen") und Liste der letzten Einsätze.

use super::{datum_uhrzeit, Ansicht};
use crate::{theme, App};
use eframe::egui::{self, CursorIcon, RichText, Sense};

/// Startbildschirm wie in der Zeichnung: großer Text in der Mitte.
/// Darunter werden laufende Einsätze zum schnellen Öffnen angezeigt.
pub fn zeigen(app: &mut App, ui: &mut egui::Ui) {
    let laufend: Vec<_> = app
        .ok(app.db.einsaetze())
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.ende.is_none())
        .collect();

    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() * 0.40);
        let text = RichText::new("Neuen Einsatz anlegen").size(40.0).strong().color(theme::TEXT_HELL);
        let antwort = ui
            .add(egui::Label::new(text).sense(Sense::click()))
            .on_hover_cursor(CursorIcon::PointingHand);
        if antwort.clicked() {
            app.f.neuer_einsatz = Default::default();
            app.ansicht = Ansicht::NeuerEinsatz;
        }

        if !laufend.is_empty() {
            ui.add_space(40.0);
            ui.label(RichText::new("Laufende Einsätze").size(20.0).color(theme::TEXT_HELL));
            for e in laufend {
                let beschriftung = format!("🔴  {} – {}   (seit {})", e.stichwort, e.ort, datum_uhrzeit(e.beginn));
                if ui.button(beschriftung).clicked() {
                    app.ansicht = Ansicht::Einsatz(e.id);
                }
            }
        }
    });
}

/// Tabelle aller bisherigen Einsätze, neueste zuerst. "Öffnen" springt in den Einsatz.
pub fn letzte_einsaetze(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Letzte Einsätze");
    ui.add_space(10.0);
    let liste = app.ok(app.db.einsaetze()).unwrap_or_default();
    if liste.is_empty() {
        ui.label("Noch keine Einsätze vorhanden.");
        return;
    }
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("einsatzliste")
            .striped(true)
            .num_columns(6)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                for kopf in ["Beginn", "Ende", "Stichwort", "Typ", "Ort", ""] {
                    ui.strong(kopf);
                }
                ui.end_row();
                for e in liste {
                    ui.label(datum_uhrzeit(e.beginn));
                    match e.ende {
                        Some(ende) => ui.label(datum_uhrzeit(ende)),
                        None => ui.colored_label(theme::ROT, "läuft"),
                    };
                    ui.label(e.stichwort.as_str());
                    ui.label(e.typ.as_str());
                    ui.label(e.ort.as_str());
                    if ui.button("Öffnen").clicked() {
                        app.ansicht = Ansicht::Einsatz(e.id);
                    }
                    ui.end_row();
                }
            });
    });
}
