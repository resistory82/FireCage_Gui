//! Einsatz-Bildschirme:
//! - `neuer_einsatz`: Formular zum Anlegen (Stichwort, Typ, Einsatzort)
//! - `zeigen`:        geöffneter Einsatz; die Unterseite (Aufgaben, Funktagebuch,
//!                    Meldetagebuch, Atemschutz) wird in der Seitenleiste gewählt.
//!                    Der Atemschutz liegt in `atemschutz.rs`.

use super::{atemschutz, auswahl_feld, datum_uhrzeit, enter_gedrueckt, optional, uhrzeit, zahl, Ansicht, EinsatzTab};
use crate::{db, theme, App};
use eframe::egui::{self, RichText};

/// Formular "Neuer Einsatz".
/// Pflicht: Stichwort, Einsatztyp, Ortsbeschreibung und entweder Koordinaten oder Adresse.
pub fn neuer_einsatz(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Neuen Einsatz anlegen");
    ui.add_space(10.0);
    let typen = app.ok(app.db.einsatztypen()).unwrap_or_default();
    let f = &mut app.f.neuer_einsatz;

    egui::Grid::new("neuer_einsatz")
        .num_columns(2)
        .spacing([14.0, 10.0])
        .show(ui, |ui| {
            ui.label("Stichwort *");
            ui.add(
                egui::TextEdit::singleline(&mut f.stichwort)
                    .desired_width(360.0)
                    .hint_text("z. B. B3 – Wohnhausbrand"),
            );
            ui.end_row();
            ui.label("Einsatztyp *");
            auswahl_feld(ui, "ne_typ", &mut f.typ, &typen, None);
            ui.end_row();
            ui.label("Einsatzort *");
            ui.add(
                egui::TextEdit::singleline(&mut f.ort)
                    .desired_width(360.0)
                    .hint_text("z. B. Wohnhaus, Waldstück nördl. Musterdorf"),
            );
            ui.end_row();

            // Koordinaten sind Pflicht, solange keine vollständige Adresse eingetragen ist.
            // Sobald Ortschaft, Straße und Hausnummer ausgefüllt sind, werden sie optional
            // (und umgekehrt: mit Koordinaten ist die Adresse optional).
            let adresse_da = [&f.ortschaft, &f.strasse, &f.hausnummer].iter().all(|t| !t.trim().is_empty());
            let koord_da = !f.breite.trim().is_empty() && !f.laenge.trim().is_empty();
            let pflicht = |optional: bool| if optional { "  (optional)" } else { " *" };

            ui.label("");
            ui.label(RichText::new("Koordinaten (WGS84) oder Adresse angeben:").italics());
            ui.end_row();
            ui.label(format!("Breitengrad{}", pflicht(adresse_da)));
            ui.add(egui::TextEdit::singleline(&mut f.breite).desired_width(160.0).hint_text("48.20849"));
            ui.end_row();
            ui.label(format!("Längengrad{}", pflicht(adresse_da)));
            ui.add(egui::TextEdit::singleline(&mut f.laenge).desired_width(160.0).hint_text("16.37208"));
            ui.end_row();

            ui.label(format!("Ortschaft{}", pflicht(koord_da)));
            ui.add(egui::TextEdit::singleline(&mut f.ortschaft).desired_width(260.0));
            ui.end_row();
            ui.label(format!("Straße{}", pflicht(koord_da)));
            ui.add(egui::TextEdit::singleline(&mut f.strasse).desired_width(260.0));
            ui.end_row();
            ui.label(format!("Hausnummer / Stiege{}", pflicht(koord_da)));
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut f.hausnummer).desired_width(80.0));
                ui.add(egui::TextEdit::singleline(&mut f.stiege).desired_width(60.0).hint_text("Stiege"));
            });
            ui.end_row();
        });

    ui.add_space(14.0);
    ui.horizontal(|ui| {
        if ui.button(RichText::new("✔ Einsatz eröffnen").size(18.0)).clicked() {
            einsatz_eroeffnen(app);
        }
        if ui.button("Abbrechen").clicked() {
            app.ansicht = Ansicht::Start;
        }
    });
}

/// Prüft das Formular "Neuer Einsatz" und legt den Einsatz in der Datenbank an.
fn einsatz_eroeffnen(app: &mut App) {
    let f = &app.f.neuer_einsatz;

    // Koordinaten: beide oder keine, und im gültigen Bereich.
    let koordinaten = match (zahl(&f.breite), zahl(&f.laenge)) {
        (Some(b), Some(l)) if (-90.0..=90.0).contains(&b) && (-180.0..=180.0).contains(&l) => Some((l, b)),
        (None, None) if f.breite.trim().is_empty() && f.laenge.trim().is_empty() => None,
        _ => {
            app.fehler("Koordinaten ungültig: Breitengrad (-90…90) und Längengrad (-180…180) beide angeben – oder beide leer lassen, wenn eine Adresse eingetragen ist.");
            return;
        }
    };

    // Adresse: nur wenn Ortschaft, Straße und Hausnummer ausgefüllt sind.
    let adresse_teile = [&f.ortschaft, &f.strasse, &f.hausnummer];
    let adresse_vollstaendig = adresse_teile.iter().all(|t| !t.trim().is_empty());
    let adresse_angefangen = adresse_teile.iter().any(|t| !t.trim().is_empty());
    if adresse_angefangen && !adresse_vollstaendig {
        app.fehler("Adresse unvollständig: Ortschaft, Straße und Hausnummer angeben.");
        return;
    }

    let Some(typ) = f.typ else {
        app.fehler("Bitte einen Einsatztyp wählen.");
        return;
    };
    if f.stichwort.trim().is_empty() || f.ort.trim().is_empty() {
        app.fehler("Bitte Stichwort und Einsatzort-Beschreibung eingeben.");
        return;
    }
    if koordinaten.is_none() && !adresse_vollstaendig {
        app.fehler("Bitte Koordinaten oder eine Adresse angeben.");
        return;
    }

    let adresse = adresse_vollstaendig.then(|| (f.ortschaft.trim(), f.strasse.trim(), f.hausnummer.trim(), optional(&f.stiege)));
    let r = app.db.einsatz_anlegen(f.stichwort.trim(), typ, f.ort.trim(), koordinaten, adresse);
    if let Some(id) = app.ok(r) {
        app.f.neuer_einsatz = Default::default();
        app.hinweis = None;
        app.ansicht = Ansicht::Einsatz(id);
    }
}

/// Geöffneter Einsatz: Kopfdaten, Reiter und der Inhalt des gewählten Reiters.
pub fn zeigen(app: &mut App, ui: &mut egui::Ui, id: i64) {
    let Some(einsatz) = app.ok(app.db.einsatz(id)).flatten() else {
        ui.label("Einsatz nicht gefunden.");
        return;
    };

    // Kopf: Stichwort, Ort, Typ, Beginn/Ende, Knopf "Einsatz beenden"
    ui.horizontal(|ui| {
        ui.heading(format!("{} – {}", einsatz.stichwort, einsatz.ort));
        ui.label(format!("{}  |  Beginn {}", einsatz.typ, datum_uhrzeit(einsatz.beginn)));
        match einsatz.ende {
            Some(ende) => {
                ui.colored_label(theme::GRUEN, format!("beendet {}", datum_uhrzeit(ende)));
            }
            None => {
                // Öffnet nur den Bestätigungsdialog – beendet wird erst dort.
                if ui.button("⏹ Einsatz beenden").clicked() {
                    app.f.beenden_dialog = true;
                }
            }
        }
    });
    ui.add_space(6.0);

    // Bestätigungsdialog "Einsatz beenden" (über allem anderen)
    if app.f.beenden_dialog && einsatz.ende.is_none() {
        beenden_dialog(app, ui.ctx(), id);
    }

    // Die Unterseiten (Aufgaben, Tagebücher, Atemschutz) werden über die
    // Menüpunkte "Einsatz" in der Seitenleiste gewählt (siehe main.rs).
    ui.separator();

    match app.einsatz_tab {
        EinsatzTab::Aufgaben => aufgaben(app, ui, id),
        EinsatzTab::Funktagebuch => funktagebuch(app, ui, id),
        EinsatzTab::Meldetagebuch => meldetagebuch(app, ui, id),
        EinsatzTab::Atemschutz => atemschutz::zeigen(app, ui, id),
        EinsatzTab::AtemschutzAbgeschlossen => atemschutz::abgeschlossen(app, ui, id),
    }
}

/// Bestätigungsdialog "Einsatz beenden".
///
/// Ein Einsatz darf nur beendet werden, wenn
/// - kein Atemschutztrupp mehr im Einsatz ist und
/// - keine Aufgabe mehr offen ist (Häkchen "Erledigt" fehlt).
/// Andernfalls listet der Dialog auf, was noch offen ist, und der
/// Knopf zum Beenden ist gesperrt.
///
/// Der Hintergrund wird abgedunkelt und blockiert Klicks, solange der Dialog offen ist.
/// Escape schließt den Dialog.
fn beenden_dialog(app: &mut App, ctx: &egui::Context, einsatz_id: i64) {
    // Was hindert am Beenden?
    let trupps: Vec<String> = app
        .ok(app.db.ats_einsaetze(einsatz_id))
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .filter(|(_, t)| t.laeuft())
        .map(|(i, t)| format!("Trupp {} – {}", i + 1, atemschutz::trupp_namen(&t.namen)))
        .collect();
    let aufgaben: Vec<String> = app
        .ok(app.db.aufgaben(einsatz_id))
        .unwrap_or_default()
        .into_iter()
        .filter(|a| !a.erledigt)
        .map(|a| format!("{}: {}", a.kategorie, a.beschreibung))
        .collect();
    let blockiert = !trupps.is_empty() || !aufgaben.is_empty();

    // Abgedunkelter Hintergrund, fängt alle Klicks ab
    egui::Area::new(egui::Id::new("dialog_hintergrund"))
        .order(egui::Order::Middle)
        .fixed_pos(egui::Pos2::ZERO)
        .show(ctx, |ui| {
            let flaeche = ctx.screen_rect();
            ui.allocate_rect(flaeche, egui::Sense::click());
            ui.painter().rect_filled(flaeche, 0.0, egui::Color32::from_black_alpha(170));
        });

    let mut schliessen = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    let mut beenden = false;

    // Dialogfenster in der Bildschirmmitte
    egui::Area::new(egui::Id::new("dialog_einsatz_beenden"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            egui::Frame::window(ui.style()).inner_margin(20.0).show(ui, |ui| {
                ui.set_max_width(520.0);
                ui.heading("Einsatz beenden");
                ui.add_space(8.0);

                if blockiert {
                    ui.colored_label(theme::ROT, RichText::new("Der Einsatz kann noch nicht beendet werden.").strong());
                    ui.add_space(6.0);
                    if !trupps.is_empty() {
                        ui.label(RichText::new(format!("Atemschutztrupps noch im Einsatz ({}):", trupps.len())).strong());
                        for t in &trupps {
                            ui.label(format!("   😷 {t}"));
                        }
                        ui.add_space(4.0);
                    }
                    if !aufgaben.is_empty() {
                        ui.label(RichText::new(format!("Offene Aufgaben ({}):", aufgaben.len())).strong());
                        // Bei sehr vielen Aufgaben nur die ersten 10 zeigen
                        for a in aufgaben.iter().take(10) {
                            ui.label(format!("   📋 {a}"));
                        }
                        if aufgaben.len() > 10 {
                            ui.label(format!("   … und {} weitere", aufgaben.len() - 10));
                        }
                    }
                    ui.add_space(6.0);
                    ui.label("Bitte zuerst alle Trupps zurückmelden und alle Aufgaben als erledigt markieren.");
                } else {
                    ui.label("Ist der Einsatz wirklich abgeschlossen?");
                    ui.label("Danach können keine Einträge mehr als laufend geführt werden.");
                }

                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    let knopf = egui::Button::new(RichText::new("⏹ Ja, Einsatz beenden").strong());
                    if ui.add_enabled(!blockiert, knopf).clicked() {
                        beenden = true;
                    }
                    if ui.button(if blockiert { "Schließen" } else { "Abbrechen" }).clicked() {
                        schliessen = true;
                    }
                });
            });
        });

    // Beenden nur, wenn nichts mehr offen ist (Zustand wird in jedem Bild neu aus der Datenbank gelesen)
    if beenden && !blockiert {
        let r = app.db.einsatz_beenden(einsatz_id);
        if app.ok(r).is_some() {
            app.info("Einsatz beendet.");
        }
        schliessen = true;
    }
    if schliessen {
        app.f.beenden_dialog = false;
    }
}

/// Reiter "Aufgaben": Abschnitte anlegen, Aufgaben anlegen, Liste mit Erledigt-Häkchen.
/// Aufgaben mit überschrittener Nachfragezeit werden rot markiert.
fn aufgaben(app: &mut App, ui: &mut egui::Ui, einsatz_id: i64) {
    let kategorien = app.ok(app.db.aufgabe_kategorien()).unwrap_or_default();
    let fahrzeuge = app.ok(app.db.fahrzeug_auswahl()).unwrap_or_default();
    let abschnitte = app.ok(app.db.abschnitte(einsatz_id)).unwrap_or_default();

    // Abschnitt anlegen
    ui.horizontal(|ui| {
        ui.label("Neuer Abschnitt");
        let feld = ui.add(
            egui::TextEdit::singleline(&mut app.f.a_neuer_abschnitt)
                .desired_width(200.0)
                .hint_text("z. B. Abschnitt Nord"),
        );
        if (ui.button("➕").clicked() || enter_gedrueckt(&feld, ui)) && !app.f.a_neuer_abschnitt.trim().is_empty() {
            let r = app.db.abschnitt_anlegen(einsatz_id, app.f.a_neuer_abschnitt.trim());
            if app.ok(r).is_some() {
                app.f.a_neuer_abschnitt.clear();
            }
        }
        if !abschnitte.is_empty() {
            let namen: Vec<&str> = abschnitte.iter().map(|a| a.name.as_str()).collect();
            ui.label(format!("Abschnitte: {}", namen.join(", ")));
        }
    });

    // Aufgabe anlegen
    ui.horizontal_wrapped(|ui| {
        auswahl_feld(ui, "a_kat", &mut app.f.a_kategorie, &kategorien, None);
        ui.add(
            egui::TextEdit::singleline(&mut app.f.a_beschreibung)
                .desired_width(320.0)
                .hint_text("Beschreibung, z. B. Innenangriff 1. OG"),
        );
        auswahl_feld(ui, "a_fz", &mut app.f.a_fahrzeug, &fahrzeuge, Some("– kein Fahrzeug –"));
        auswahl_feld(ui, "a_ab", &mut app.f.a_abschnitt, &abschnitte, Some("– kein Abschnitt –"));
        ui.label("Nachfragen in");
        ui.add(egui::TextEdit::singleline(&mut app.f.a_nachfragen_min).desired_width(40.0));
        ui.label("min");
        if ui.button("➕ Aufgabe").clicked() {
            aufgabe_anlegen(app, einsatz_id);
        }
    });
    ui.separator();

    // Liste der Aufgaben
    let liste = app.ok(app.db.aufgaben(einsatz_id)).unwrap_or_default();
    let jetzt = db::jetzt();
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("aufgaben")
            .striped(true)
            .num_columns(7)
            .spacing([18.0, 6.0])
            .show(ui, |ui| {
                for kopf in [
                    "Zeit",
                    "Kategorie",
                    "Beschreibung",
                    "Fahrzeug",
                    "Abschnitt",
                    "Nachfragen",
                    "Erledigt",
                ] {
                    ui.strong(kopf);
                }
                ui.end_row();
                for a in &liste {
                    ui.label(uhrzeit(a.zeitpunkt));
                    ui.colored_label(theme::hex_farbe(a.farbe.as_deref()), format!("■ {}", a.kategorie));
                    ui.label(a.beschreibung.as_str());
                    ui.label(a.fahrzeug.as_deref().unwrap_or("–"));
                    ui.label(a.abschnitt.as_deref().unwrap_or("–"));
                    match a.nachfragen_um {
                        Some(t) if !a.erledigt && t <= jetzt => ui.colored_label(theme::ROT, format!("⚠ {} fällig", uhrzeit(t))),
                        Some(t) => ui.label(uhrzeit(t)),
                        None => ui.label("–"),
                    };
                    let mut erledigt = a.erledigt;
                    if ui.checkbox(&mut erledigt, "").changed() {
                        let r = app.db.aufgabe_erledigt(a.id, erledigt);
                        app.ok(r);
                    }
                    ui.end_row();
                }
            });
    });
}

/// Prüft das Aufgaben-Formular und speichert die Aufgabe.
fn aufgabe_anlegen(app: &mut App, einsatz_id: i64) {
    let f = &app.f;
    let Some(kat) = f.a_kategorie else {
        app.fehler("Bitte eine Kategorie wählen.");
        return;
    };
    if f.a_beschreibung.trim().is_empty() {
        app.fehler("Bitte eine Beschreibung eingeben.");
        return;
    }
    // Nachfragezeit: Minuten ab jetzt, leer = keine Nachfrage
    let nachfragen_um = match optional(&f.a_nachfragen_min) {
        None => None,
        Some(t) => match t.parse::<i64>() {
            Ok(min) if min > 0 => Some(db::jetzt() + min * 60),
            _ => {
                app.fehler("Nachfragezeit bitte als ganze Minuten angeben.");
                return;
            }
        },
    };
    let r = app
        .db
        .aufgabe_anlegen(einsatz_id, kat, f.a_beschreibung.trim(), f.a_fahrzeug, f.a_abschnitt, nachfragen_um);
    if app.ok(r).is_some() {
        app.f.a_beschreibung.clear();
        app.f.a_nachfragen_min.clear();
    }
}

/// Reiter "Funktagebuch": Funkspruch eintragen (Enter im Inhaltsfeld genügt) und Liste.
fn funktagebuch(app: &mut App, ui: &mut egui::Ui, einsatz_id: i64) {
    let sprechgruppen = app.ok(app.db.sprechgruppen()).unwrap_or_default();
    if sprechgruppen.is_empty() {
        ui.label("Bitte zuerst unter „Sprechgruppen“ mindestens eine Sprechgruppe anlegen.");
        return;
    }

    let mut eintragen = false;
    ui.horizontal_wrapped(|ui| {
        auswahl_feld(ui, "fu_sg", &mut app.f.fu_sprechgruppe, &sprechgruppen, None);
        ui.add(
            egui::TextEdit::singleline(&mut app.f.fu_rufstelle)
                .desired_width(150.0)
                .hint_text("Rufstelle (von)"),
        );
        ui.add(
            egui::TextEdit::singleline(&mut app.f.fu_meldestelle)
                .desired_width(150.0)
                .hint_text("Meldestelle (an)"),
        );
        let feld = ui.add(
            egui::TextEdit::singleline(&mut app.f.fu_inhalt)
                .desired_width(400.0)
                .hint_text("Inhalt des Funkspruchs"),
        );
        eintragen = ui.button("Eintragen").clicked() || enter_gedrueckt(&feld, ui);
    });
    if eintragen {
        let f = &app.f;
        match f.fu_sprechgruppe {
            None => app.fehler("Bitte eine Sprechgruppe wählen."),
            Some(_) if f.fu_rufstelle.trim().is_empty() || f.fu_meldestelle.trim().is_empty() || f.fu_inhalt.trim().is_empty() => {
                app.fehler("Bitte Rufstelle, Meldestelle und Inhalt eingeben.")
            }
            Some(sg) => {
                let r = app
                    .db
                    .funkspruch_anlegen(einsatz_id, sg, f.fu_rufstelle.trim(), f.fu_meldestelle.trim(), f.fu_inhalt.trim());
                if app.ok(r).is_some() {
                    // Rufstelle/Meldestelle bleiben stehen, oft funkt dieselbe Stelle mehrmals.
                    app.f.fu_inhalt.clear();
                }
            }
        }
    }
    ui.separator();

    let liste = app.ok(app.db.funksprueche(einsatz_id)).unwrap_or_default();
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("funk")
            .striped(true)
            .num_columns(5)
            .spacing([18.0, 6.0])
            .show(ui, |ui| {
                for kopf in ["Zeit", "Sprechgruppe", "Von", "An", "Inhalt"] {
                    ui.strong(kopf);
                }
                ui.end_row();
                for e in &liste {
                    ui.label(uhrzeit(e.zeit));
                    ui.label(e.sprechgruppe.as_str());
                    ui.label(e.rufstelle.as_str());
                    ui.label(e.meldestelle.as_str());
                    ui.label(e.inhalt.as_str());
                    ui.end_row();
                }
            });
    });
}

/// Reiter "Meldetagebuch": persönliche Befehle/Meldungen eintragen und Liste.
fn meldetagebuch(app: &mut App, ui: &mut egui::Ui, einsatz_id: i64) {
    let kategorien = app.ok(app.db.meldung_kategorien()).unwrap_or_default();

    let mut eintragen = false;
    ui.horizontal_wrapped(|ui| {
        auswahl_feld(ui, "m_kat", &mut app.f.m_kategorie, &kategorien, None);
        ui.add(egui::TextEdit::singleline(&mut app.f.m_von).desired_width(150.0).hint_text("Von"));
        ui.add(egui::TextEdit::singleline(&mut app.f.m_an).desired_width(150.0).hint_text("An"));
        let feld = ui.add(
            egui::TextEdit::singleline(&mut app.f.m_inhalt)
                .desired_width(400.0)
                .hint_text("Inhalt"),
        );
        eintragen = ui.button("Eintragen").clicked() || enter_gedrueckt(&feld, ui);
    });
    if eintragen {
        let f = &app.f;
        match f.m_kategorie {
            None => app.fehler("Bitte eine Kategorie wählen."),
            Some(_) if f.m_von.trim().is_empty() || f.m_an.trim().is_empty() || f.m_inhalt.trim().is_empty() => {
                app.fehler("Bitte Von, An und Inhalt eingeben.")
            }
            Some(k) => {
                let r = app
                    .db
                    .meldung_anlegen(einsatz_id, k, f.m_von.trim(), f.m_an.trim(), f.m_inhalt.trim());
                if app.ok(r).is_some() {
                    app.f.m_inhalt.clear();
                }
            }
        }
    }
    ui.separator();

    let liste = app.ok(app.db.meldungen(einsatz_id)).unwrap_or_default();
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("meld")
            .striped(true)
            .num_columns(5)
            .spacing([18.0, 6.0])
            .show(ui, |ui| {
                for kopf in ["Zeit", "Art", "Von", "An", "Inhalt"] {
                    ui.strong(kopf);
                }
                ui.end_row();
                for e in &liste {
                    ui.label(uhrzeit(e.zeit));
                    ui.label(e.kategorie.as_str());
                    ui.label(e.von.as_str());
                    ui.label(e.an.as_str());
                    ui.label(e.inhalt.as_str());
                    ui.end_row();
                }
            });
    });
}
