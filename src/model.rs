//! Datenstrukturen, wie sie aus der Datenbank gelesen und in der Oberfläche
//! angezeigt werden. Zeitangaben sind immer Unix-Sekunden.

/// Eintrag für Auswahllisten (Feuerwehr, Kategorie, Sprechgruppe …): id + Anzeigename.
#[derive(Clone)]
pub struct Auswahl {
    pub id: i64,
    pub name: String,
}

/// Eine Feuerwehr (Stammdaten).
pub struct Feuerwehr {
    pub id: i64,
    pub name: String,
    pub kurzname: Option<String>,
    pub aktiv: bool,
}

/// Ein Fahrzeug aus dem Fahrzeugpark, inkl. Name der Feuerwehr.
pub struct Fahrzeug {
    pub id: i64,
    pub feuerwehr: String,
    pub rufname: String,
    pub kurzname: String,
    pub aktiv: bool,
}

/// Eine Person aus dem Personalstand, inkl. Qualifikationen als Text ("ATS, GRKDT").
pub struct Person {
    pub id: i64,
    pub feuerwehr: String,
    pub vorname: String,
    pub nachname: String,
    pub rang: String,
    pub aktiv: bool,
    pub qualifikationen: String,
}

/// Kopfdaten eines Einsatzes für Listen und die Einsatzansicht.
pub struct EinsatzInfo {
    pub id: i64,
    pub stichwort: String,
    pub typ: String,
    pub beginn: i64,
    pub ende: Option<i64>,
    /// Beschreibung des ersten Einsatzorts.
    pub ort: String,
}

/// Eine Aufgabe im Einsatz.
pub struct Aufgabe {
    pub id: i64,
    pub zeitpunkt: i64,
    pub erledigt: bool,
    pub nachfragen_um: Option<i64>,
    pub beschreibung: String,
    pub kategorie: String,
    pub farbe: Option<String>,
    pub fahrzeug: Option<String>,
    pub abschnitt: Option<String>,
    /// Einsatzort als Anzeigetext, z. B. "Wohnhaus (Hauptstraße 12, Musterdorf)".
    pub ort: Option<String>,
}

/// Ein Eintrag im Funktagebuch.
pub struct Funkspruch {
    pub zeit: i64,
    pub sprechgruppe: String,
    pub rufstelle: String,
    pub meldestelle: String,
    pub inhalt: String,
}

/// Ein Eintrag im Meldetagebuch (persönliche Befehle/Meldungen).
pub struct Meldung {
    pub zeit: i64,
    pub kategorie: String,
    pub von: String,
    pub an: String,
    pub inhalt: String,
}

/// Ein Atemschutztrupp im Einsatz.
/// Rollen-Index überall: 0 = ATF (Truppführer), 1 = ATM (Truppmann), 2 = WTF.
pub struct AtsEinsatz {
    pub id: i64,
    /// Frei wählbarer Truppname (Standard beim Anlegen: "Trupp 1", "Trupp 2", …).
    pub truppname: String,
    /// Druckabfrage als durchgeführt markiert (Kästchen neben "Druckabfrage").
    pub druckabfrage: bool,
    /// Zeitpunkt "Trupp geht vor"; `None` = Trupp wartet noch.
    pub start: Option<i64>,
    /// Zeitpunkt "Trupp zurück"; `None` = noch nicht zurück.
    pub ende: Option<i64>,
    /// Maximale Einsatzzeit in Sekunden.
    pub einsatzzeit: i64,
    /// Namen je Rolle (leer = nicht besetzt).
    pub namen: [String; 3],
    /// Drücke in bar je Rolle: [Anfangsdruck, Druckabfrage, Enddruck].
    pub druck: [[Option<i64>; 3]; 3],
}

impl AtsEinsatz {
    /// Trupp ist vorgegangen und noch nicht zurück.
    pub fn laeuft(&self) -> bool {
        self.start.is_some() && self.ende.is_none()
    }
}
