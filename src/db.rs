//! Datenbankzugriff (SQLite über rusqlite).
//!
//! Das Schema wird direkt aus `Database/FirecageDatabaseScema.sql` eingebettet
//! (dein drawDB-Export). Beim Start werden alle Tabellen mit
//! `CREATE TABLE IF NOT EXISTS` angelegt und leere Auswahltabellen mit
//! Standardwerten befüllt.
//!
//! Hinweis: Ein neuer Export ergänzt bestehende Tabellen NICHT um neue Spalten.
//! Während der Entwicklung bei Schemaänderungen einfach `firecage.db` löschen.

use crate::model::*;
use rusqlite::{params, Connection, OptionalExtension, Params, Result};

/// Das Datenbankschema, beim Kompilieren aus der SQL-Datei eingelesen.
const SCHEMA: &str = include_str!("../Database/FirecageDatabaseScema.sql");

/// Aktuelle Zeit als Unix-Sekunden (so werden alle Zeiten gespeichert).
pub fn jetzt() -> i64 {
    chrono::Local::now().timestamp()
}

/// Erzeugt eine neue, zufällige UUID für einen Datensatz.
fn neue_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Kapselt die Verbindung zur Datenbank. Alle Abfragen laufen über diese Struktur.
pub struct Db {
    conn: Connection,
}

impl Db {
    /// Öffnet (oder erstellt) die Datenbankdatei, aktiviert Fremdschlüssel,
    /// legt die Tabellen an und befüllt Standardwerte.
    pub fn oeffnen(pfad: &str) -> Result<Self> {
        let conn = Connection::open(pfad)?;
        // SQLite prüft Fremdschlüssel nur, wenn das pro Verbindung eingeschaltet wird.
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.execute_batch(SCHEMA)?;
        let db = Self { conn };
        db.standardwerte_anlegen()?;
        Ok(db)
    }

    /// Anzahl der Zeilen einer Tabelle (nur mit festen Tabellennamen aufrufen).
    fn anzahl(&self, tabelle: &str) -> Result<i64> {
        self.conn
            .query_row(&format!("SELECT COUNT(*) FROM \"{tabelle}\""), [], |r| r.get(0))
    }

    /// Befüllt leere Auswahltabellen mit sinnvollen Startwerten.
    /// Diese können später in der Datenbank angepasst werden.
    fn standardwerte_anlegen(&self) -> Result<()> {
        if self.anzahl("Einsatztyp")? == 0 {
            for n in [
                "Brandeinsatz",
                "Technischer Einsatz",
                "Schadstoffeinsatz",
                "Unwetter",
                "Großschadenslage KHD",
                "Sonstiges",
            ] {
                self.conn.execute("INSERT INTO Einsatztyp (Name) VALUES (?1)", params![n])?;
            }
        }
        if self.anzahl("AufgabeKategorie")? == 0 {
            let kategorien = [
                //Allgemein
                ("Anfahrt", "#E6A01E"),
                ("Bereitstellung warten", "#6FA8DC"),
                ("Erkundung", "#B4A7D6"),
                //ATS Spezifisch
                ("ATS Menschenrettung", "#E06666"),
                ("ATS Brandbekämpfung", "#E06666"),
                ("ATS Schadstoff", "#E06666"),
                ("Atemschutzsammelplatz", "#A64D79"),
                // Brand allgemein
                ("Brandbekämpfung", "#CC0000"),
                ("Wasserversorgung", "#45818E"),
                // Technisch allgemein
                ("Technischer Einsatz", "#3D85C6"),
                ("Absicherung/Verkehr", "#F1C232"),
                //Allgemein
                ("Rückbau/Aufräumen", "#93C47D"),
                ("Einsatzbereit am Einsatzort", "#6AA84F"),
                ("Abgerückt", "#999999"),
                ("Storniert", "#999999"),
            ];
            for (i, (name, farbe)) in kategorien.iter().enumerate() {
                self.conn.execute(
                    "INSERT INTO AufgabeKategorie (Name, Farbe, Sortierung, aktiv) VALUES (?1, ?2, ?3, 1)",
                    params![name, farbe, i as i64],
                )?;
            }
        }
        if self.anzahl("MeldungKategorie")? == 0 {
            for n in ["Befehl", "Meldung", "Rückfrage", "Information"] {
                self.conn.execute("INSERT INTO MeldungKategorie (Name) VALUES (?1)", params![n])?;
            }
        }
        if self.anzahl("Qualifikation")? == 0 {
            let qualis = [
                ("ATS", "Atemschutzgeräteträger"),
                ("GRKDT", "Gruppenkommandant"),
                ("ZGKDT", "Zugskommandant"),
                ("MA", "Maschinist"),
                ("FU", "Funker"),
                ("SAN", "Sanitäter"),
            ];
            for (kurz, voll) in qualis {
                self.conn.execute(
                    "INSERT INTO Qualifikation (uuid, Kurzname, Vollname) VALUES (?1, ?2, ?3)",
                    params![neue_uuid(), kurz, voll],
                )?;
            }
        }
        Ok(())
    }

    /// Allgemeine Abfrage für Auswahllisten. Die SQL-Abfrage muss `id, name` liefern.
    pub fn auswahl<P: Params>(&self, sql: &str, p: P) -> Result<Vec<Auswahl>> {
        let mut stmt = self.conn.prepare(sql)?;
        let zeilen = stmt.query_map(p, |r| {
            Ok(Auswahl {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })?;
        zeilen.collect()
    }

    /// Setzt das Feld `aktiv` (Stammdaten werden nie gelöscht, nur deaktiviert).
    /// `tabelle` darf nur ein fester Tabellenname aus dem Code sein.
    pub fn aktiv_setzen(&self, tabelle: &str, id: i64, aktiv: bool) -> Result<()> {
        self.conn.execute(
            &format!("UPDATE \"{tabelle}\" SET aktiv = ?1 WHERE id = ?2"),
            params![aktiv as i64, id],
        )?;
        Ok(())
    }

    // ───────────────────────────── Feuerwehr ─────────────────────────────

    /// Alle Feuerwehren, alphabetisch.
    pub fn feuerwehren(&self) -> Result<Vec<Feuerwehr>> {
        let mut stmt = self.conn.prepare("SELECT id, Name, Kurzname, aktiv FROM Feuerwehr ORDER BY Name")?;
        let zeilen = stmt.query_map([], |r| {
            Ok(Feuerwehr {
                id: r.get(0)?,
                name: r.get(1)?,
                kurzname: r.get(2)?,
                aktiv: r.get::<_, i64>(3)? == 1,
            })
        })?;
        zeilen.collect()
    }

    /// Nur aktive Feuerwehren als Auswahlliste (für Fahrzeuge/Personal).
    pub fn feuerwehr_auswahl(&self) -> Result<Vec<Auswahl>> {
        self.auswahl("SELECT id, Name FROM Feuerwehr WHERE aktiv = 1 ORDER BY Name", [])
    }

    /// Legt eine neue Feuerwehr an.
    pub fn feuerwehr_anlegen(&self, name: &str, kurzname: Option<&str>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO Feuerwehr (uuid, aktiv, Name, Kurzname) VALUES (?1, 1, ?2, ?3)",
            params![neue_uuid(), name, kurzname],
        )?;
        Ok(())
    }

    // ───────────────────────────── Fahrzeuge ─────────────────────────────

    /// Alle Fahrzeuge mit Feuerwehr-Name.
    pub fn fahrzeuge(&self) -> Result<Vec<Fahrzeug>> {
        let mut stmt = self.conn.prepare(
            "SELECT f.id, fw.Name, f.Rufname, f.Kurzname, f.aktiv
             FROM Fahrzeuge f JOIN Feuerwehr fw ON fw.id = f.FeuerwehrID
             ORDER BY fw.Name, f.Rufname",
        )?;
        let zeilen = stmt.query_map([], |r| {
            Ok(Fahrzeug {
                id: r.get(0)?,
                feuerwehr: r.get(1)?,
                rufname: r.get(2)?,
                kurzname: r.get(3)?,
                aktiv: r.get::<_, i64>(4)? == 1,
            })
        })?;
        zeilen.collect()
    }

    /// Aktive Fahrzeuge als Auswahlliste (für Aufgaben).
    pub fn fahrzeug_auswahl(&self) -> Result<Vec<Auswahl>> {
        self.auswahl("SELECT id, Rufname FROM Fahrzeuge WHERE aktiv = 1 ORDER BY Rufname", [])
    }

    /// Legt ein neues Fahrzeug an.
    pub fn fahrzeug_anlegen(&self, feuerwehr_id: i64, rufname: &str, kurzname: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO Fahrzeuge (uuid, FeuerwehrID, Rufname, Kurzname, aktiv) VALUES (?1, ?2, ?3, ?4, 1)",
            params![neue_uuid(), feuerwehr_id, rufname, kurzname],
        )?;
        Ok(())
    }

    // ───────────────────────────── Personal ─────────────────────────────

    /// Gesamter Personalstand inkl. Qualifikationen (als Text zusammengefasst).
    pub fn personal(&self) -> Result<Vec<Person>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, fw.Name, p.Vorname, p.Nachname, p.Rang, p.aktiv,
                    COALESCE((SELECT group_concat(q.Kurzname, ', ')
                              FROM PersonalQualifikation pq JOIN Qualifikation q ON q.id = pq.QualifikationID
                              WHERE pq.PersonalID = p.id), '')
             FROM Personal p JOIN Feuerwehr fw ON fw.id = p.FeuerwehrID
             ORDER BY p.Nachname, p.Vorname",
        )?;
        let zeilen = stmt.query_map([], |r| {
            Ok(Person {
                id: r.get(0)?,
                feuerwehr: r.get(1)?,
                vorname: r.get(2)?,
                nachname: r.get(3)?,
                rang: r.get(4)?,
                aktiv: r.get::<_, i64>(5)? == 1,
                qualifikationen: r.get(6)?,
            })
        })?;
        zeilen.collect()
    }

    /// Alle Qualifikationen als Auswahlliste ("ATS – Atemschutzgeräteträger").
    pub fn qualifikationen(&self) -> Result<Vec<Auswahl>> {
        self.auswahl("SELECT id, Kurzname || ' – ' || Vollname FROM Qualifikation ORDER BY Kurzname", [])
    }

    /// Legt eine Person samt Qualifikationen an (in einer Transaktion: alles oder nichts).
    pub fn person_anlegen(&self, feuerwehr_id: i64, vorname: &str, nachname: &str, rang: &str, qualis: &[i64]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO Personal (uuid, FeuerwehrID, aktiv, Vorname, Nachname, Rang) VALUES (?1, ?2, 1, ?3, ?4, ?5)",
            params![neue_uuid(), feuerwehr_id, vorname, nachname, rang],
        )?;
        let person_id = tx.last_insert_rowid();
        for q in qualis {
            tx.execute(
                "INSERT INTO PersonalQualifikation (PersonalID, QualifikationID) VALUES (?1, ?2)",
                params![person_id, q],
            )?;
        }
        tx.commit()
    }

    // ───────────────────────────── Sprechgruppen ─────────────────────────────

    /// Alle Sprechgruppen als Auswahlliste.
    pub fn sprechgruppen(&self) -> Result<Vec<Auswahl>> {
        self.auswahl("SELECT id, Name FROM Sprechgruppe ORDER BY Name", [])
    }

    /// Legt eine neue Sprechgruppe an.
    pub fn sprechgruppe_anlegen(&self, name: &str) -> Result<()> {
        self.conn
            .execute("INSERT INTO Sprechgruppe (uuid, Name) VALUES (?1, ?2)", params![neue_uuid(), name])?;
        Ok(())
    }

    // ───────────────────────────── Einsätze ─────────────────────────────

    /// Liest Einsätze; mit `Some(id)` nur diesen einen, sonst alle (neueste zuerst).
    fn einsaetze_lesen(&self, nur_id: Option<i64>) -> Result<Vec<EinsatzInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.Stichwort, t.Name, e.Einsatzbeginn, e.Einsatzende,
                    COALESCE((SELECT o.Beschreibung FROM Einsatzort o WHERE o.EinsatzID = e.id ORDER BY o.id LIMIT 1), '')
             FROM Einsatz e JOIN Einsatztyp t ON t.id = e.EinsatztypID
             WHERE ?1 IS NULL OR e.id = ?1
             ORDER BY e.Einsatzbeginn DESC",
        )?;
        let zeilen = stmt.query_map(params![nur_id], |r| {
            Ok(EinsatzInfo {
                id: r.get(0)?,
                stichwort: r.get(1)?,
                typ: r.get(2)?,
                beginn: r.get(3)?,
                ende: r.get(4)?,
                ort: r.get(5)?,
            })
        })?;
        zeilen.collect()
    }

    /// Alle Einsätze, neueste zuerst.
    pub fn einsaetze(&self) -> Result<Vec<EinsatzInfo>> {
        self.einsaetze_lesen(None)
    }

    /// Ein einzelner Einsatz.
    pub fn einsatz(&self, id: i64) -> Result<Option<EinsatzInfo>> {
        Ok(self.einsaetze_lesen(Some(id))?.into_iter().next())
    }

    /// id des jüngsten noch laufenden Einsatzes (für die Einsatzdauer auf der Startseite).
    pub fn laufender_einsatz(&self) -> Result<Option<i64>> {
        self.conn
            .query_row(
                "SELECT id FROM Einsatz WHERE Einsatzende IS NULL ORDER BY Einsatzbeginn DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()
    }

    /// Einsatztypen als Auswahlliste.
    pub fn einsatztypen(&self) -> Result<Vec<Auswahl>> {
        self.auswahl("SELECT id, Name FROM Einsatztyp ORDER BY id", [])
    }

    /// Legt einen neuen Einsatz samt erstem Einsatzort (und optionaler Adresse) an.
    /// Alles in einer Transaktion – schlägt ein Schritt fehl, wird nichts gespeichert.
    /// Gibt die id des neuen Einsatzes zurück.
    pub fn einsatz_anlegen(
        &self,
        stichwort: &str,
        typ_id: i64,
        ort_beschreibung: &str,
        koordinaten: Option<(f64, f64)>,
        adresse: Option<(&str, &str, &str, Option<&str>)>,
    ) -> Result<i64> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO Einsatz (uuid, EinsatztypID, Einsatzbeginn, Stichwort) VALUES (?1, ?2, ?3, ?4)",
            params![neue_uuid(), typ_id, jetzt(), stichwort],
        )?;
        let einsatz_id = tx.last_insert_rowid();

        let adresse_id = match adresse {
            Some((ortschaft, strasse, hausnummer, stiege)) => {
                tx.execute(
                    "INSERT INTO Adresse (uuid, Ortschaft, Strasse, Hausnummer, Stiege) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![neue_uuid(), ortschaft, strasse, hausnummer, stiege],
                )?;
                Some(tx.last_insert_rowid())
            }
            None => None,
        };

        let (laenge, breite) = match koordinaten {
            Some((l, b)) => (Some(l), Some(b)),
            None => (None, None),
        };
        tx.execute(
            "INSERT INTO Einsatzort (Beschreibung, Laengengrad, Breitengrad, AdresseID, EinsatzID) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![ort_beschreibung, laenge, breite, adresse_id, einsatz_id],
        )?;
        tx.commit()?;
        Ok(einsatz_id)
    }

    /// Setzt das Einsatzende auf jetzt.
    pub fn einsatz_beenden(&self, id: i64) -> Result<()> {
        self.conn
            .execute("UPDATE Einsatz SET Einsatzende = ?1 WHERE id = ?2", params![jetzt(), id])?;
        Ok(())
    }

    // ───────────────────────────── Abschnitte ─────────────────────────────

    /// Einsatzabschnitte eines Einsatzes.
    pub fn abschnitte(&self, einsatz_id: i64) -> Result<Vec<Auswahl>> {
        self.auswahl(
            "SELECT id, Name FROM Einsatzabschnitt WHERE EinsatzID = ?1 ORDER BY id",
            params![einsatz_id],
        )
    }

    /// Legt einen Einsatzabschnitt an.
    pub fn abschnitt_anlegen(&self, einsatz_id: i64, name: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO Einsatzabschnitt (EinsatzID, Name) VALUES (?1, ?2)",
            params![einsatz_id, name],
        )?;
        Ok(())
    }

    // ───────────────────────────── Aufgaben ─────────────────────────────

    /// Aktive Aufgaben-Kategorien (sortiert).
    pub fn aufgabe_kategorien(&self) -> Result<Vec<Auswahl>> {
        self.auswahl("SELECT id, Name FROM AufgabeKategorie WHERE aktiv = 1 ORDER BY Sortierung", [])
    }

    /// Alle Aufgaben eines Einsatzes: offene zuerst, dann neueste zuerst.
    pub fn aufgaben(&self, einsatz_id: i64) -> Result<Vec<Aufgabe>> {
        let mut stmt = self.conn.prepare(
            "SELECT a.id, a.Zeitpunkt, a.Erledigt, a.NachfragenUm, a.Beschreibung, k.Name, k.Farbe, f.Rufname, s.Name
             FROM Aufgaben a
             JOIN AufgabeKategorie k ON k.id = a.KategorieID
             LEFT JOIN Fahrzeuge f ON f.id = a.FahrzeugID
             LEFT JOIN Einsatzabschnitt s ON s.id = a.AbschnittID
             WHERE a.EinsatzID = ?1
             ORDER BY a.Erledigt, a.Zeitpunkt DESC",
        )?;
        let zeilen = stmt.query_map(params![einsatz_id], |r| {
            Ok(Aufgabe {
                id: r.get(0)?,
                zeitpunkt: r.get(1)?,
                erledigt: r.get::<_, i64>(2)? == 1,
                nachfragen_um: r.get(3)?,
                beschreibung: r.get(4)?,
                kategorie: r.get(5)?,
                farbe: r.get(6)?,
                fahrzeug: r.get(7)?,
                abschnitt: r.get(8)?,
            })
        })?;
        zeilen.collect()
    }

    /// Legt eine Aufgabe an. Der Einsatzort wird automatisch auf den ersten Ort des Einsatzes gesetzt.
    pub fn aufgabe_anlegen(
        &self,
        einsatz_id: i64,
        kategorie_id: i64,
        beschreibung: &str,
        fahrzeug_id: Option<i64>,
        abschnitt_id: Option<i64>,
        nachfragen_um: Option<i64>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO Aufgaben (EinsatzID, AbschnittID, FahrzeugID, Zeitpunkt, Erledigt, NachfragenUm, EinsatzortID, Beschreibung, KategorieID)
             VALUES (?1, ?2, ?3, ?4, 0, ?5,
                     (SELECT id FROM Einsatzort WHERE EinsatzID = ?1 ORDER BY id LIMIT 1),
                     ?6, ?7)",
            params![einsatz_id, abschnitt_id, fahrzeug_id, jetzt(), nachfragen_um, beschreibung, kategorie_id],
        )?;
        Ok(())
    }

    /// Markiert eine Aufgabe als erledigt bzw. wieder offen.
    pub fn aufgabe_erledigt(&self, id: i64, erledigt: bool) -> Result<()> {
        self.conn
            .execute("UPDATE Aufgaben SET Erledigt = ?1 WHERE id = ?2", params![erledigt as i64, id])?;
        Ok(())
    }

    // ───────────────────────────── Funktagebuch ─────────────────────────────

    /// Funksprüche eines Einsatzes, neueste zuerst.
    pub fn funksprueche(&self, einsatz_id: i64) -> Result<Vec<Funkspruch>> {
        let mut stmt = self.conn.prepare(
            "SELECT f.EingegangenUm, s.Name, f.Rufstelle, f.Meldestelle, f.Inhalt
             FROM Funktagebuch f JOIN Sprechgruppe s ON s.id = f.SprechgruppeID
             WHERE f.EinsatzID = ?1 ORDER BY f.EingegangenUm DESC, f.id DESC",
        )?;
        let zeilen = stmt.query_map(params![einsatz_id], |r| {
            Ok(Funkspruch {
                zeit: r.get(0)?,
                sprechgruppe: r.get(1)?,
                rufstelle: r.get(2)?,
                meldestelle: r.get(3)?,
                inhalt: r.get(4)?,
            })
        })?;
        zeilen.collect()
    }

    /// Trägt einen Funkspruch mit aktueller Uhrzeit ein.
    pub fn funkspruch_anlegen(
        &self,
        einsatz_id: i64,
        sprechgruppe_id: i64,
        rufstelle: &str,
        meldestelle: &str,
        inhalt: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO Funktagebuch (EinsatzID, EingegangenUm, Inhalt, SprechgruppeID, Rufstelle, Meldestelle)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![einsatz_id, jetzt(), inhalt, sprechgruppe_id, rufstelle, meldestelle],
        )?;
        Ok(())
    }

    // ───────────────────────────── Meldetagebuch ─────────────────────────────

    /// Kategorien für das Meldetagebuch.
    pub fn meldung_kategorien(&self) -> Result<Vec<Auswahl>> {
        self.auswahl("SELECT id, Name FROM MeldungKategorie ORDER BY id", [])
    }

    /// Meldungen eines Einsatzes, neueste zuerst.
    pub fn meldungen(&self, einsatz_id: i64) -> Result<Vec<Meldung>> {
        let mut stmt = self.conn.prepare(
            "SELECT m.EingegangenUm, k.Name, m.Von, m.An, m.Inhalt
             FROM Meldetagebuch m JOIN MeldungKategorie k ON k.id = m.KategorieID
             WHERE m.EinsatzID = ?1 ORDER BY m.EingegangenUm DESC, m.id DESC",
        )?;
        let zeilen = stmt.query_map(params![einsatz_id], |r| {
            Ok(Meldung {
                zeit: r.get(0)?,
                kategorie: r.get(1)?,
                von: r.get(2)?,
                an: r.get(3)?,
                inhalt: r.get(4)?,
            })
        })?;
        zeilen.collect()
    }

    /// Trägt eine Meldung mit aktueller Uhrzeit ein.
    pub fn meldung_anlegen(&self, einsatz_id: i64, kategorie_id: i64, von: &str, an: &str, inhalt: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO Meldetagebuch (EinsatzID, KategorieID, Inhalt, EingegangenUm, Von, An) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![einsatz_id, kategorie_id, inhalt, jetzt(), von, an],
        )?;
        Ok(())
    }

    // ───────────────────────────── Atemschutz ─────────────────────────────

    /// Spaltennamen der Atemschutz-Tabelle je Rolle (0 = ATF, 1 = ATM, 2 = WTF):
    /// [Name, Anfangsdruck, Druckabfrage, Enddruck]
    const ATS_SPALTEN: [[&'static str; 4]; 3] = [
        ["ATFName", "DruckATFStart", "DruckATFNachfrage", "DruckATFEnde"],
        ["ATMName", "DruckATMStart", "DruckATMNachfrage", "DruckATMEnde"],
        ["WTFName", "DruckWTFStart", "DruckWTFNachfrage", "DruckWTFEnde"],
    ];

    /// Alle Atemschutztrupps eines Einsatzes in Anlege-Reihenfolge
    /// (daraus ergibt sich die Trupp-Nummer 1, 2, 3 …).
    pub fn ats_einsaetze(&self, einsatz_id: i64) -> Result<Vec<AtsEinsatz>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, EinsatzStart, AbgeschlossenUm, COALESCE(Einsatzzeit, 1800),
                    COALESCE(ATFName, ''), DruckATFStart, DruckATFNachfrage, DruckATFEnde,
                    COALESCE(ATMName, ''), DruckATMStart, DruckATMNachfrage, DruckATMEnde,
                    COALESCE(WTFName, ''), DruckWTFStart, DruckWTFNachfrage, DruckWTFEnde
             FROM Atemschutzeinsatz WHERE EinsatzID = ?1 ORDER BY id",
        )?;
        let zeilen = stmt.query_map(params![einsatz_id], |r| {
            Ok(AtsEinsatz {
                id: r.get(0)?,
                start: r.get(1)?,
                ende: r.get(2)?,
                einsatzzeit: r.get(3)?,
                namen: [r.get(4)?, r.get(8)?, r.get(12)?],
                druck: [
                    [r.get(5)?, r.get(6)?, r.get(7)?],
                    [r.get(9)?, r.get(10)?, r.get(11)?],
                    [r.get(13)?, r.get(14)?, r.get(15)?],
                ],
            })
        })?;
        zeilen.collect()
    }

    /// Legt einen neuen, noch wartenden Trupp an (EinsatzStart = leer).
    /// `ATSTruppID` ist im Schema Pflicht, zeigt aber auf keine Tabelle – daher 0.
    pub fn ats_neu(&self, einsatz_id: i64) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO Atemschutzeinsatz (ATSTruppID, EinsatzID, Einsatzzeit, Erledigt) VALUES (0, ?1, 1800, 0)",
            params![einsatz_id],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Speichert den Namen eines Truppmitglieds (`rolle` 0..3). Leerer Text = NULL.
    pub fn ats_name_setzen(&self, id: i64, rolle: usize, name: &str) -> Result<()> {
        let spalte = Self::ATS_SPALTEN[rolle][0];
        let wert = if name.trim().is_empty() { None } else { Some(name.trim()) };
        self.conn.execute(
            &format!("UPDATE Atemschutzeinsatz SET {spalte} = ?1 WHERE id = ?2"),
            params![wert, id],
        )?;
        Ok(())
    }

    /// Speichert einen Druckwert. `art`: 1 = Anfangsdruck, 2 = Druckabfrage, 3 = Enddruck.
    pub fn ats_druck_setzen(&self, id: i64, rolle: usize, art: usize, bar: Option<i64>) -> Result<()> {
        let spalte = Self::ATS_SPALTEN[rolle][art];
        self.conn.execute(
            &format!("UPDATE Atemschutzeinsatz SET {spalte} = ?1 WHERE id = ?2"),
            params![bar, id],
        )?;
        Ok(())
    }

    /// Setzt die maximale Einsatzzeit in Sekunden.
    pub fn ats_einsatzzeit_setzen(&self, id: i64, sekunden: i64) -> Result<()> {
        self.conn
            .execute("UPDATE Atemschutzeinsatz SET Einsatzzeit = ?1 WHERE id = ?2", params![sekunden, id])?;
        Ok(())
    }

    /// Trupp geht vor: Startzeit = jetzt (nur wenn noch nicht gestartet).
    pub fn ats_starten(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE Atemschutzeinsatz SET EinsatzStart = ?1 WHERE id = ?2 AND EinsatzStart IS NULL",
            params![jetzt(), id],
        )?;
        Ok(())
    }

    /// Löscht einen Trupp, der noch nicht vorgegangen ist (gestartete bleiben zur Dokumentation).
    pub fn ats_verwerfen(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM Atemschutzeinsatz WHERE id = ?1 AND EinsatzStart IS NULL", params![id])?;
        Ok(())
    }

    /// Beendet einen Atemschutzeinsatz (Trupp zurück).
    pub fn ats_beenden(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE Atemschutzeinsatz SET AbgeschlossenUm = ?1, Erledigt = 1 WHERE id = ?2",
            params![jetzt(), id],
        )?;
        Ok(())
    }
}
