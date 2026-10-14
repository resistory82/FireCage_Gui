//! Firecage – Einsatzleitung für Feuerwehren
//!
//! Einstiegspunkt der Anwendung. Hier werden das Fenster geöffnet, die Datenbank
//! verbunden und das Grundlayout (Kopfzeile, Seitenleiste, Hauptbereich) gezeichnet.
//! Die einzelnen Bildschirme liegen im Modul `ansichten`.

// Im Release-Build unter Windows kein Konsolenfenster öffnen.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ansichten;
mod db;
mod model;
mod stammdaten;
mod theme;

use ansichten::{Ansicht, EinsatzTab, Formulare};
use eframe::egui::{self, Align, Layout, RichText, Sense};

/// Dateiname der SQLite-Datenbank (liegt im Arbeitsverzeichnis).
const DB_DATEI: &str = "firecage.db";

/// Gesamter Zustand der Anwendung.
pub struct App {
    /// Verbindung zur Datenbank.
    pub db: db::Db,
    /// Welcher Bildschirm gerade angezeigt wird.
    pub ansicht: Ansicht,
    /// Meldung oben im Hauptbereich: (Text, ist_fehler).
    pub hinweis: Option<(String, bool)>,
    /// Inhalte aller Eingabeformulare (bleiben beim Wechsel der Ansicht erhalten).
    pub f: Formulare,
    /// Gewählte Unterseite des Einsatzes (Aufgaben, Tagebücher, Atemschutz).
    pub einsatz_tab: EinsatzTab,
    /// Zuletzt geöffneter Einsatz – bestimmt, für welchen Einsatz die
    /// Menüpunkte "Einsatz" in der Seitenleiste gelten.
    pub letzter_einsatz: Option<i64>,
    /// Eingeblendete Erinnerungen für fällige Aufgaben (siehe ansichten/erinnerung.rs).
    pub erinnerungen: ansichten::erinnerung::Erinnerungen,
}

impl App {
    fn neu(db: db::Db) -> Self {
        Self {
            db,
            ansicht: Ansicht::Start,
            hinweis: None,
            f: Formulare::default(),
            einsatz_tab: EinsatzTab::Aufgaben,
            letzter_einsatz: None,
            erinnerungen: Default::default(),
        }
    }

    /// Wertet ein Datenbank-Ergebnis aus: Bei Erfolg wird der Wert zurückgegeben,
    /// bei einem Fehler wird er als roter Hinweis angezeigt und `None` geliefert.
    pub fn ok<T>(&mut self, r: rusqlite::Result<T>) -> Option<T> {
        match r {
            Ok(v) => Some(v),
            Err(e) => {
                self.fehler(format!("Datenbankfehler: {e}"));
                None
            }
        }
    }

    /// Zeigt einen roten Fehlerhinweis an.
    pub fn fehler(&mut self, text: impl Into<String>) {
        self.hinweis = Some((text.into(), true));
    }

    /// Zeigt einen grünen Erfolgshinweis an.
    pub fn info(&mut self, text: impl Into<String>) {
        self.hinweis = Some((text.into(), false));
    }

    /// Welcher Einsatz gerade "im Fokus" ist:
    /// 1. der geöffnete Einsatz, sonst
    /// 2. der zuletzt geöffnete, falls er noch läuft, sonst
    /// 3. der jüngste laufende Einsatz.
    /// Ist keiner vorhanden, werden die Einsatz-Menüpunkte ausgeblendet.
    fn einsatz_im_fokus(&self) -> Option<i64> {
        if let Ansicht::Einsatz(id) = self.ansicht {
            return Some(id);
        }
        let laeuft = |id: i64| self.db.einsatz(id).ok().flatten().is_some_and(|e| e.ende.is_none());
        match self.letzter_einsatz {
            Some(id) if laeuft(id) => Some(id),
            _ => self.db.laufender_einsatz().ok().flatten(),
        }
    }

    /// Berechnet den Text für die Einsatzdauer rechts oben (Einsatz im Fokus, sonst "--:--:--").
    fn einsatzdauer(&mut self) -> String {
        let id = self.einsatz_im_fokus();
        let Some(id) = id else { return "--:--:--".into() };
        match self.db.einsatz(id) {
            Ok(Some(e)) => {
                let sek = (e.ende.unwrap_or_else(db::jetzt) - e.beginn).max(0);
                format!("{:02}:{:02}:{:02}", sek / 3600, sek / 60 % 60, sek % 60)
            }
            _ => "--:--:--".into(),
        }
    }

    /// Kopfzeile: Logo links (Klick = Startseite), Einsatzdauer rechts.
    fn kopfzeile(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("kopf")
            .exact_height(120.0)
            .frame(
                egui::Frame::none()
                    .fill(theme::KOPF)
                    .inner_margin(egui::Margin::symmetric(24.0, 8.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let logo = egui::Image::from_bytes("bytes://firecage_logo.svg", theme::LOGO)
                        .fit_to_exact_size(egui::vec2(100.0, 100.0))
                        .sense(Sense::click());
                    if ui.add(logo).on_hover_text("Zur Startseite").clicked() {
                        self.ansicht = Ansicht::Start;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let dauer = self.einsatzdauer();
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Einsatzdauer").size(30.0).strong().color(theme::TEXT_HELL));
                            ui.label(RichText::new(dauer).size(40.0).strong().color(theme::ROT));
                        });
                        ui.separator();
                    });
                });
            });
    }

    /// Seitenleiste links: Konfigurationsmenü oben, Version + Uhrzeit unten.
    fn seitenleiste(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("menue")
            .exact_width(170.0)
            .resizable(false)
            .frame(egui::Frame::none().fill(theme::SEITE))
            .show(ctx, |ui| {
                // Unterer weißer Kasten mit Version und aktueller Uhrzeit.
                egui::TopBottomPanel::bottom("uhr")
                    .frame(egui::Frame::none().fill(egui::Color32::WHITE).inner_margin(6.0))
                    .show_inside(ui, |ui| {
                        ui.label(
                            RichText::new(format!("Firecage alpha V{}", env!("CARGO_PKG_VERSION")))
                                .size(10.0)
                                .color(egui::Color32::GRAY),
                        );
                        let jetzt = chrono::Local::now().format("%d.%m.%Y %H:%M:%S").to_string();
                        ui.label(RichText::new(jetzt).size(15.0).strong().color(theme::TEXT_DUNKEL));
                    });

                // Menü "Konfiguration".
                egui::Frame::none().inner_margin(10.0).show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Konfiguration").size(19.0).strong().color(theme::TEXT_HELL));
                    });
                    ui.add_space(6.0);
                    let eintraege = [
                        ("Feuerwehr", Ansicht::Feuerwehr),
                        ("Fahrzeugpark", Ansicht::Fahrzeugpark),
                        ("Personal", Ansicht::Personal),
                        ("Sprechgruppen", Ansicht::Sprechgruppen),
                        ("Letzte Einsätze", Ansicht::LetzteEinsaetze),
                    ];
                    for (text, ziel) in eintraege {
                        if theme::menue_knopf(ui, text, self.ansicht == ziel).clicked() {
                            self.ansicht = ziel;
                            self.hinweis = None;
                        }
                        ui.add_space(4.0);
                    }
                });
                ui.separator();

                // Menü "Einsatz": nur sichtbar, solange ein Einsatz läuft bzw. geöffnet ist.
                if let Some(einsatz_id) = self.einsatz_im_fokus() {
                    egui::Frame::none().inner_margin(10.0).show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("Einsatz").size(19.0).strong().color(theme::TEXT_HELL));
                        });
                        ui.add_space(6.0);
                        let eintraege = [
                            ("Aufgaben", EinsatzTab::Aufgaben),
                            ("Meldetagebuch", EinsatzTab::Meldetagebuch),
                            ("Funktagebuch", EinsatzTab::Funktagebuch),
                            ("Atemschutz", EinsatzTab::Atemschutz),
                            ("Atemschutz\nAbgeschlossen", EinsatzTab::AtemschutzAbgeschlossen),
                        ];
                        for (text, tab) in eintraege {
                            let aktiv = self.ansicht == Ansicht::Einsatz(einsatz_id) && self.einsatz_tab == tab;
                            if theme::menue_knopf(ui, text, aktiv).clicked() {
                                self.ansicht = Ansicht::Einsatz(einsatz_id);
                                self.einsatz_tab = tab;
                                self.hinweis = None;
                            }
                            ui.add_space(4.0);
                        }
                    });
                    ui.separator();
                }
            });
    }

    /// Zeigt den aktuellen Hinweis (Fehler rot / Erfolg grün) mit Schließen-Knopf.
    fn hinweis_anzeigen(&mut self, ui: &mut egui::Ui) {
        let Some((text, ist_fehler)) = self.hinweis.clone() else { return };
        let farbe = if ist_fehler { theme::ROT } else { theme::GRUEN };
        let mut schliessen = false;
        egui::Frame::none()
            .stroke(egui::Stroke::new(1.5, farbe))
            .rounding(6.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(text).color(farbe));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        schliessen = ui.small_button("✖").clicked();
                    });
                });
            });
        if schliessen {
            self.hinweis = None;
        }
        ui.add_space(8.0);
    }
}

impl eframe::App for App {
    /// Wird für jedes Bild aufgerufen und zeichnet die gesamte Oberfläche.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Mindestens 2x pro Sekunde neu zeichnen, damit Uhr und Einsatzdauer laufen.
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // Geöffneten Einsatz merken (für das Einsatz-Menü in der Seitenleiste).
        if let Ansicht::Einsatz(id) = self.ansicht {
            self.letzter_einsatz = Some(id);
        }

        // Reihenfolge wichtig: Panels am Rand zuerst, der Hauptbereich zuletzt.
        self.kopfzeile(ctx);
        self.seitenleiste(ctx);

        // Trupp-Leiste unten, nur auf den Atemschutz-Seiten.
        if let Ansicht::Einsatz(id) = self.ansicht {
            if matches!(self.einsatz_tab, EinsatzTab::Atemschutz | EinsatzTab::AtemschutzAbgeschlossen) {
                ansichten::atemschutz::trupp_leiste(self, ctx, id);
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::HINTERGRUND).inner_margin(20.0))
            .show(ctx, |ui| {
                self.hinweis_anzeigen(ui);
                match self.ansicht {
                    Ansicht::Start => ansichten::start::zeigen(self, ui),
                    Ansicht::Feuerwehr => ansichten::konfiguration::feuerwehr(self, ui),
                    Ansicht::Fahrzeugpark => ansichten::konfiguration::fahrzeugpark(self, ui),
                    Ansicht::Personal => ansichten::konfiguration::personal(self, ui),
                    Ansicht::Sprechgruppen => ansichten::konfiguration::sprechgruppen(self, ui),
                    Ansicht::LetzteEinsaetze => ansichten::start::letzte_einsaetze(self, ui),
                    Ansicht::NeuerEinsatz => ansichten::einsatz::neuer_einsatz(self, ui),
                    Ansicht::Einsatz(id) => ansichten::einsatz::zeigen(self, ui, id),
                }
            });

        // Fällige Aufgaben prüfen und als blinkende Erinnerung einblenden (auf jeder Seite)
        ansichten::erinnerung::pruefen(self, ctx);
        ansichten::erinnerung::zeigen(self, ctx);

        // Import-Vorschau für Stammdaten (erscheint nur, wenn eine Datei geladen wurde)
        ansichten::austausch::dialog(self, ctx);

        // Runder "+"-Knopf unten rechts (nur auf Start und in der Einsatzliste).
        if matches!(self.ansicht, Ansicht::Start | Ansicht::LetzteEinsaetze) && theme::plus_knopf(ctx) {
            self.f.neuer_einsatz = Default::default();
            self.ansicht = Ansicht::NeuerEinsatz;
            self.hinweis = None;
        }
    }
}

/// Ersatz-Oberfläche, falls die Datenbank nicht geöffnet werden konnte.
struct FehlerApp(String);

impl eframe::App for FehlerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Firecage konnte nicht gestartet werden");
            ui.colored_label(theme::ROT, &self.0);
        });
    }
}

fn main() -> eframe::Result<()> {
    // Fenster: startet maximiert (volle Bildschirmgröße, Taskleiste bleibt sichtbar).
    // `with_inner_size` gilt nur, wenn man das Fenster später "wiederherstellt".
    let mut fenster = egui::ViewportBuilder::default()
        .with_title("Firecage")
        .with_maximized(true)
        .with_inner_size([1600.0, 900.0])
        .with_min_inner_size([1024.0, 640.0]);

    // Fenster- und Taskleistensymbol: Firecage-Logo als PNG (in die .exe eingebettet).
    // Schlägt das Laden fehl, startet das Programm trotzdem mit dem Standardsymbol.
    match eframe::icon_data::from_png_bytes(theme::FENSTER_ICON) {
        Ok(icon) => fenster = fenster.with_icon(icon),
        Err(e) => eprintln!("Fenstersymbol konnte nicht geladen werden: {e}"),
    }

    let optionen = eframe::NativeOptions {
        viewport: fenster,
        ..Default::default()
    };
    eframe::run_native(
        "Firecage",
        optionen,
        Box::new(|cc| {
            theme::anwenden(&cc.egui_ctx);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            let app: Box<dyn eframe::App> = match db::Db::oeffnen(DB_DATEI) {
                Ok(db) => Box::new(App::neu(db)),
                Err(e) => Box::new(FehlerApp(format!("Datenbank '{DB_DATEI}' konnte nicht geöffnet werden: {e}"))),
            };
            Ok(app)
        }),
    )
}
