//! Erinnerungen (blinkende Popups oben in der Mitte, auf jeder Seite) für:
//! - **Aufgaben**, deren Nachfragezeit verstrichen ist
//! - **Atemschutz: Druckabfrage fällig** – ab 1/3 der Einsatzzeit, solange nicht für
//!   alle Truppmitglieder ein Druckabfrage-Wert eingetragen ist
//! - **Atemschutz: Einsatzzeit überschritten** – bleibt stehen, bis sie weggeklickt
//!   oder der Trupp zurückgemeldet wird
//!
//! `pruefen()` läuft mit jedem Bild (ca. 2× pro Sekunde) und erzeugt für jeden Anlass
//! genau EINE Erinnerung. Ist der Anlass erledigt (Aufgabe abgehakt, Druck eingetragen,
//! Trupp zurück), verschwindet die Erinnerung von selbst.

use super::atemschutz::{druckabfrage_vollstaendig, mmss, trupp_namen, vergangen};
use super::{Ansicht, EinsatzTab};
use crate::{db, theme, App};
use eframe::egui::{self, Color32, RichText, Stroke};
use std::collections::HashSet;

/// Wie lange eine (nicht dauerhafte) Erinnerung sichtbar bleibt (Sekunden).
const ANZEIGEDAUER: f64 = 15.0;
/// Höchstens so viele Erinnerungen gleichzeitig anzeigen.
const MAX_SICHTBAR: usize = 4;
/// Verlängerungsstufen in Minuten (auch in der Aufgabentabelle verwendet).
pub const VERLAENGERUNG_MIN: [i64; 4] = [5, 10, 15, 30];

/// Worauf sich eine Erinnerung bezieht.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Anlass {
    /// Nachfragezeit einer Aufgabe verstrichen (Aufgaben-id)
    Aufgabe(i64),
    /// Druckabfrage eines Atemschutztrupps fällig (Trupp-id)
    Druckabfrage(i64),
    /// Einsatzzeit eines Atemschutztrupps überschritten (Trupp-id)
    AtsZeit(i64),
}

/// Eine eingeblendete Erinnerung.
pub struct Erinnerung {
    pub anlass: Anlass,
    pub einsatz_id: i64,
    pub titel: &'static str,
    pub text: String,
    /// Zeitpunkt (egui-Zeit in Sekunden), ab dem sie angezeigt wird.
    pub seit: f64,
    /// `true` = bleibt stehen, bis sie weggeklickt wird oder der Anlass erledigt ist.
    pub dauerhaft: bool,
}

/// Aktive Erinnerungen und bereits gemeldete Anlässe.
/// `gemeldet` enthält (Anlass, Zusatzwert): bei Aufgaben die Nachfragezeit – nach einer
/// Verlängerung meldet sich die Aufgabe daher erneut.
#[derive(Default)]
pub struct Erinnerungen {
    pub aktiv: Vec<Erinnerung>,
    pub gemeldet: HashSet<(Anlass, i64)>,
}

/// Ein aktuell fälliger Anlass (Zwischenergebnis von `pruefen`).
struct Faellig {
    anlass: Anlass,
    zusatz: i64,
    einsatz_id: i64,
    titel: &'static str,
    text: String,
    dauerhaft: bool,
}

/// Sucht fällige Aufgaben und Atemschutz-Anlässe und legt dafür Erinnerungen an.
pub fn pruefen(app: &mut App, ctx: &egui::Context) {
    let zeit = ctx.input(|i| i.time);
    let jetzt = db::jetzt();
    let mut faellig: Vec<Faellig> = Vec::new();

    // Aufgaben
    if let Ok(liste) = app.db.faellige_aufgaben() {
        for (aufgabe_id, einsatz_id, nachfragen_um, text) in liste {
            faellig.push(Faellig {
                anlass: Anlass::Aufgabe(aufgabe_id),
                zusatz: nachfragen_um,
                einsatz_id,
                titel: "⏰ Nachfrage fällig",
                text,
                dauerhaft: false,
            });
        }
    }

    // Atemschutz: alle laufenden Trupps aller laufenden Einsätze
    for einsatz_id in app.db.laufende_einsaetze().unwrap_or_default() {
        let trupps = app.db.ats_einsaetze(einsatz_id).unwrap_or_default();
        for t in trupps.iter().filter(|t| t.laeuft()) {
            let sek = vergangen(t, jetzt);
            let anteil = sek as f64 / t.einsatzzeit.max(1) as f64;
            let text = format!(
                "{} – {}   ·   {} / {} min",
                t.truppname,
                trupp_namen(&t.namen),
                mmss(sek),
                t.einsatzzeit / 60
            );
            if anteil >= 1.0 {
                faellig.push(Faellig {
                    anlass: Anlass::AtsZeit(t.id),
                    zusatz: t.einsatzzeit,
                    einsatz_id,
                    titel: "⚠ Atemschutz-Einsatzzeit überschritten",
                    text,
                    dauerhaft: true,
                });
            } else if anteil >= 1.0 / 3.0 && !druckabfrage_vollstaendig(t) {
                faellig.push(Faellig {
                    anlass: Anlass::Druckabfrage(t.id),
                    zusatz: 0,
                    einsatz_id,
                    titel: "⚠ Atemschutz: Druckabfrage fällig",
                    text,
                    dauerhaft: false,
                });
            }
        }
    }

    let aktuell: HashSet<Anlass> = faellig.iter().map(|f| f.anlass).collect();
    let e = &mut app.erinnerungen;
    for f in faellig {
        if e.gemeldet.insert((f.anlass, f.zusatz)) {
            // Neu fällig → (ältere Erinnerung desselben Anlasses ersetzen) und anzeigen
            e.aktiv.retain(|x| x.anlass != f.anlass);
            e.aktiv.push(Erinnerung {
                anlass: f.anlass,
                einsatz_id: f.einsatz_id,
                titel: f.titel,
                text: f.text,
                seit: zeit,
                dauerhaft: f.dauerhaft,
            });
        } else if let Some(x) = e.aktiv.iter_mut().find(|x| x.anlass == f.anlass) {
            // Schon sichtbar → nur den Text aktualisieren (z. B. laufende Atemschutzzeit)
            x.text = f.text;
        }
    }
    // Ausblenden, wenn der Anlass erledigt ist oder die Anzeigedauer abgelaufen ist
    e.aktiv
        .retain(|x| aktuell.contains(&x.anlass) && (x.dauerhaft || zeit - x.seit < ANZEIGEDAUER));
}

/// Was in einer Erinnerung geklickt wurde.
enum Aktion {
    Verlaengern(i64, i64),
    Erledigt(i64),
    ZurAufgabe(Anlass, i64),
    /// (Anlass, Einsatz-id, Trupp-id, Cursor in Druckabfrage setzen)
    ZumTrupp(Anlass, i64, i64, bool),
    Schliessen(Anlass),
}

/// Zeichnet die Erinnerungen oben in der Bildschirmmitte (über allen Ansichten und Dialogen).
pub fn zeigen(app: &mut App, ctx: &egui::Context) {
    if app.erinnerungen.aktiv.is_empty() {
        return;
    }
    // Für flüssiges Blinken öfter neu zeichnen, solange Erinnerungen sichtbar sind
    ctx.request_repaint_after(std::time::Duration::from_millis(100));
    let zeit = ctx.input(|i| i.time);
    // Blinkender Rahmen: wechselt zweimal pro Sekunde zwischen kräftig und schwach
    let blink = ((zeit * std::f64::consts::TAU * 2.0).sin() * 0.5 + 0.5) as f32;
    let rahmen = Color32::from_rgb(255, (90.0 + 120.0 * blink) as u8, (60.0 + 80.0 * blink) as u8);

    let mut aktionen: Vec<Aktion> = Vec::new();
    let anzahl = app.erinnerungen.aktiv.len();

    egui::Area::new(egui::Id::new("erinnerungen"))
        .order(egui::Order::Tooltip) // liegt auch über Popups
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 130.0))
        .show(ctx, |ui| {
            // Dauerhafte (kritische) Erinnerungen zuerst
            let mut sortiert: Vec<&Erinnerung> = app.erinnerungen.aktiv.iter().collect();
            sortiert.sort_by_key(|e| !e.dauerhaft);
            for e in sortiert.into_iter().take(MAX_SICHTBAR) {
                let hintergrund = if e.dauerhaft {
                    Color32::from_rgb(110, 25, 25)
                } else {
                    Color32::from_rgb(70, 30, 30)
                };
                egui::Frame::none()
                    .fill(hintergrund)
                    .stroke(Stroke::new(3.0, rahmen))
                    .rounding(8.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_width(600.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(e.titel).size(20.0).strong().color(Color32::WHITE));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("✖").on_hover_text("Ausblenden").clicked() {
                                    aktionen.push(Aktion::Schliessen(e.anlass));
                                }
                            });
                        });
                        ui.label(RichText::new(&e.text).size(17.0).color(theme::TEXT_HELL));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| match e.anlass {
                            Anlass::Aufgabe(id) => {
                                ui.label("Verlängern:");
                                for m in VERLAENGERUNG_MIN {
                                    if ui.button(format!("+{m} min")).clicked() {
                                        aktionen.push(Aktion::Verlaengern(id, m));
                                    }
                                }
                                ui.separator();
                                if ui.button("✔ Erledigt").clicked() {
                                    aktionen.push(Aktion::Erledigt(id));
                                }
                                if ui.button("Zur Aufgabe").clicked() {
                                    aktionen.push(Aktion::ZurAufgabe(e.anlass, e.einsatz_id));
                                }
                            }
                            Anlass::Druckabfrage(trupp) => {
                                if ui.button("Druckabfrage eintragen").clicked() {
                                    aktionen.push(Aktion::ZumTrupp(e.anlass, e.einsatz_id, trupp, true));
                                }
                            }
                            Anlass::AtsZeit(trupp) => {
                                if ui.button("Zur Atemschutzüberwachung").clicked() {
                                    aktionen.push(Aktion::ZumTrupp(e.anlass, e.einsatz_id, trupp, false));
                                }
                            }
                        });
                        // Restzeit als dünner Balken (nur bei nicht dauerhaften Erinnerungen)
                        if !e.dauerhaft {
                            let rest = (1.0 - (zeit - e.seit) / ANZEIGEDAUER).clamp(0.0, 1.0) as f32;
                            ui.add(egui::ProgressBar::new(rest).fill(rahmen));
                        }
                    });
                ui.add_space(6.0);
            }
            if anzahl > MAX_SICHTBAR {
                ui.label(RichText::new(format!("… und {} weitere Erinnerungen", anzahl - MAX_SICHTBAR)).color(Color32::WHITE));
            }
        });

    for a in aktionen {
        match a {
            Aktion::Verlaengern(id, m) => {
                let r = app.db.aufgabe_verlaengern(id, m);
                if app.ok(r).is_some() {
                    entfernen(app, Anlass::Aufgabe(id));
                }
            }
            Aktion::Erledigt(id) => {
                let r = app.db.aufgabe_erledigt(id, true);
                if app.ok(r).is_some() {
                    entfernen(app, Anlass::Aufgabe(id));
                }
            }
            Aktion::ZurAufgabe(anlass, einsatz_id) => {
                app.ansicht = Ansicht::Einsatz(einsatz_id);
                app.einsatz_tab = EinsatzTab::Aufgaben;
                entfernen(app, anlass);
            }
            Aktion::ZumTrupp(anlass, einsatz_id, trupp, fokus) => {
                app.ansicht = Ansicht::Einsatz(einsatz_id);
                app.einsatz_tab = EinsatzTab::Atemschutz;
                app.f.ats_gewaehlt = Some(trupp);
                app.f.ats_tabelle = false;
                app.f.ats_fokus_abfrage = fokus;
                entfernen(app, anlass);
            }
            Aktion::Schliessen(anlass) => entfernen(app, anlass),
        }
    }
}

/// Blendet eine Erinnerung aus (sie kommt erst bei einem neuen Anlass wieder).
fn entfernen(app: &mut App, anlass: Anlass) {
    app.erinnerungen.aktiv.retain(|e| e.anlass != anlass);
}
