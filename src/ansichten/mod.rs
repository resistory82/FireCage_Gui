//! Alle Bildschirme der Anwendung plus gemeinsame Hilfsfunktionen.
//!
//! - `start`:          Startbildschirm und Liste der letzten Einsätze
//! - `konfiguration`:  Stammdaten (Feuerwehr, Fahrzeugpark, Personal, Sprechgruppen)
//! - `einsatz`:        Neuen Einsatz anlegen und laufenden Einsatz führen

pub mod atemschutz;
pub mod austausch;
pub mod einsatz;
pub mod erinnerung;
pub mod konfiguration;
pub mod start;

use crate::model::Auswahl;
use chrono::TimeZone;
use eframe::egui;

/// Welcher Bildschirm im Hauptbereich angezeigt wird.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Ansicht {
    Start,
    Feuerwehr,
    Fahrzeugpark,
    Personal,
    Sprechgruppen,
    LetzteEinsaetze,
    NeuerEinsatz,
    /// Geöffneter Einsatz mit seiner id.
    Einsatz(i64),
}

/// Reiter innerhalb der Einsatzansicht.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EinsatzTab {
    Aufgaben,
    Funktagebuch,
    Meldetagebuch,
    Atemschutz,
    AtemschutzAbgeschlossen,
}

/// Eingabefelder des Formulars "Neuer Einsatz".
#[derive(Default)]
pub struct NeuerEinsatzFormular {
    pub stichwort: String,
    pub typ: Option<i64>,
    pub ort: String,
    pub laenge: String,
    pub breite: String,
    pub ortschaft: String,
    pub strasse: String,
    pub hausnummer: String,
    pub stiege: String,
}

/// Inhalte aller Eingabeformulare. Bleiben erhalten, wenn man die Ansicht wechselt.
#[derive(Default)]
pub struct Formulare {
    // Feuerwehr
    pub fw_name: String,
    pub fw_kurz: String,
    // Fahrzeug
    pub fz_feuerwehr: Option<i64>,
    pub fz_rufname: String,
    pub fz_kurz: String,
    // Personal
    pub p_feuerwehr: Option<i64>,
    pub p_vorname: String,
    pub p_nachname: String,
    pub p_rang: String,
    pub p_qualis: Vec<i64>,
    // Sprechgruppe
    pub sg_name: String,
    // Neuer Einsatz
    pub neuer_einsatz: NeuerEinsatzFormular,
    // Aufgabe
    pub a_kategorie: Option<i64>,
    pub a_beschreibung: String,
    pub a_fahrzeug: Option<i64>,
    pub a_abschnitt: Option<i64>,
    pub a_nachfragen_min: String,
    pub a_neuer_abschnitt: String,
    // Funktagebuch
    pub fu_sprechgruppe: Option<i64>,
    pub fu_rufstelle: String,
    pub fu_meldestelle: String,
    pub fu_inhalt: String,
    // Meldetagebuch
    pub m_kategorie: Option<i64>,
    pub m_von: String,
    pub m_an: String,
    pub m_inhalt: String,
    // Atemschutz
    /// id des Trupps, der in der Einzelansicht angezeigt wird.
    pub ats_gewaehlt: Option<i64>,
    /// `true` = Tabellenansicht aller Trupps, `false` = Einzelansicht.
    pub ats_tabelle: bool,
    /// Eingabetexte der Trupp-Tabelle je Trupp-id: [Rolle][Name, Anfangsdruck, Druckabfrage, Enddruck].
    /// Gespeichert wird bei jeder gültigen Änderung; der Text bleibt hier, damit Tippen flüssig geht.
    pub ats_puffer: std::collections::HashMap<i64, [[String; 4]; 3]>,
    /// Eingabetext des Truppnamens je Trupp-id.
    pub ats_name_puffer: std::collections::HashMap<i64, String>,
    /// Eingabetext "max. Einsatzzeit (min)" je Trupp-id.
    pub ats_zeit_puffer: std::collections::HashMap<i64, String>,
    /// Popup "Atemschutzträger wählen", solange geöffnet.
    pub ats_auswahl: Option<atemschutz::TraegerAuswahl>,
    /// Nach Klick auf "Druckabfrage": Cursor ins erste Druckabfrage-Feld setzen.
    pub ats_fokus_abfrage: bool,
    // Import-Vorschau (Stammdaten), solange der Dialog offen ist
    pub import: Option<austausch::ImportVorschau>,
    // Popup "Einsatzort zuordnen" für eine Aufgabe
    pub ort_dialog: Option<einsatz::OrtDialog>,
    // Bestätigungsdialog "Einsatz beenden" ist geöffnet
    pub beenden_dialog: bool,
}

/// Auswahlfeld (Dropdown) für eine Liste aus der Datenbank.
/// `leer` ist der Text für "nichts gewählt"; ist er `None`, muss etwas gewählt werden.
pub fn auswahl_feld(ui: &mut egui::Ui, id: &str, wert: &mut Option<i64>, liste: &[Auswahl], leer: Option<&str>) {
    let anzeige = wert
        .and_then(|w| liste.iter().find(|a| a.id == w))
        .map(|a| a.name.clone())
        .unwrap_or_else(|| leer.unwrap_or("– bitte wählen –").to_string());
    egui::ComboBox::from_id_salt(id)
        .selected_text(anzeige)
        .width(220.0)
        .show_ui(ui, |ui| {
            if let Some(t) = leer {
                ui.selectable_value(wert, None, t);
            }
            for a in liste {
                ui.selectable_value(wert, Some(a.id), a.name.as_str());
            }
        });
}

/// Unix-Sekunden → "HH:MM:SS" (Ortszeit).
pub fn uhrzeit(ts: i64) -> String {
    chrono::Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| d.format("%H:%M:%S").to_string())
        .unwrap_or_default()
}

/// Unix-Sekunden → "TT.MM.JJJJ HH:MM" (Ortszeit).
pub fn datum_uhrzeit(ts: i64) -> String {
    chrono::Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| d.format("%d.%m.%Y %H:%M").to_string())
        .unwrap_or_default()
}

/// Liest eine Kommazahl aus einem Textfeld; akzeptiert "," und ".".
pub fn zahl(text: &str) -> Option<f64> {
    text.trim().replace(',', ".").parse().ok()
}

/// Gibt `None` für leere Textfelder zurück, sonst den getrimmten Text.
pub fn optional(text: &str) -> Option<&str> {
    let t = text.trim();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

/// Prüft, ob in einem Textfeld gerade Enter gedrückt wurde (zum schnellen Eintragen).
pub fn enter_gedrueckt(antwort: &egui::Response, ui: &egui::Ui) -> bool {
    antwort.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
}
