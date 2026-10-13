//! Import/Export der Stammdaten (Feuerwehr, Fahrzeugpark, Personal mit
//! Qualifikationen, Sprechgruppen) als eine gemeinsame JSON-Datei.
//!
//! Ablauf beim Import:
//! 1. `lesen()`        – Datei einlesen und Format/Version prüfen
//! 2. `analysieren()`  – jeden Datensatz mit der Datenbank vergleichen (Konfliktprüfung)
//! 3. `importieren()`  – nach den gewählten Optionen in EINER Transaktion übernehmen
//!
//! Datensätze werden über ihre UUID wiedererkannt, weil die `id`-Zahlen in jeder
//! Datenbank anders sind. Qualifikationen werden über den Kurznamen ("ATS")
//! zugeordnet, da die Standard-Qualifikationen in jeder Datenbank eigene UUIDs haben.

use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Kennung und Version des Dateiformats (für spätere Formatänderungen).
pub const FORMAT: &str = "firecage-stammdaten";
pub const VERSION: u32 = 1;

// ───────────────────────────── Dateiformat ─────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct FeuerwehrJ {
    pub uuid: String,
    pub name: String,
    pub kurzname: Option<String>,
    pub aktiv: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FahrzeugJ {
    pub uuid: String,
    /// UUID der Feuerwehr, zu der das Fahrzeug gehört.
    pub feuerwehr: String,
    pub rufname: String,
    pub kurzname: String,
    pub aktiv: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct QualifikationJ {
    pub uuid: String,
    pub kurzname: String,
    pub vollname: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PersonJ {
    pub uuid: String,
    /// UUID der Feuerwehr, zu der die Person gehört.
    pub feuerwehr: String,
    pub vorname: String,
    pub nachname: String,
    pub rang: String,
    pub aktiv: bool,
    /// Kurznamen der Qualifikationen, z. B. ["ATS", "MA"].
    #[serde(default)]
    pub qualifikationen: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SprechgruppeJ {
    pub uuid: String,
    pub name: String,
    pub sortierung1: Option<String>,
    pub sortierung2: Option<String>,
    pub sortierung3: Option<String>,
}

/// Inhalt einer Export-Datei.
#[derive(Serialize, Deserialize, Clone)]
pub struct Paket {
    pub format: String,
    pub version: u32,
    pub exportiert_am: String,
    #[serde(default)]
    pub feuerwehren: Vec<FeuerwehrJ>,
    #[serde(default)]
    pub qualifikationen: Vec<QualifikationJ>,
    #[serde(default)]
    pub fahrzeuge: Vec<FahrzeugJ>,
    #[serde(default)]
    pub personal: Vec<PersonJ>,
    #[serde(default)]
    pub sprechgruppen: Vec<SprechgruppeJ>,
}

impl Paket {
    /// Kurzbeschreibung für Meldungen: "3 Feuerwehren, 12 Fahrzeuge, …"
    pub fn zusammenfassung(&self) -> String {
        format!(
            "{} Feuerwehren, {} Fahrzeuge, {} Personen, {} Qualifikationen, {} Sprechgruppen",
            self.feuerwehren.len(),
            self.fahrzeuge.len(),
            self.personal.len(),
            self.qualifikationen.len(),
            self.sprechgruppen.len()
        )
    }
}

// ───────────────────────────── Export ─────────────────────────────

/// Liest alle Stammdaten aus der Datenbank (auch deaktivierte Einträge).
pub fn exportieren(conn: &Connection) -> Result<Paket> {
    let mut feuerwehren = Vec::new();
    let mut s = conn.prepare("SELECT uuid, Name, Kurzname, aktiv FROM Feuerwehr ORDER BY Name")?;
    for z in s.query_map([], |r| {
        Ok(FeuerwehrJ {
            uuid: r.get(0)?,
            name: r.get(1)?,
            kurzname: r.get(2)?,
            aktiv: r.get::<_, i64>(3)? == 1,
        })
    })? {
        feuerwehren.push(z?);
    }

    let mut qualifikationen = Vec::new();
    let mut s = conn.prepare("SELECT uuid, Kurzname, Vollname FROM Qualifikation ORDER BY Kurzname")?;
    for z in s.query_map([], |r| {
        Ok(QualifikationJ {
            uuid: r.get(0)?,
            kurzname: r.get(1)?,
            vollname: r.get(2)?,
        })
    })? {
        qualifikationen.push(z?);
    }

    let mut fahrzeuge = Vec::new();
    let mut s = conn.prepare(
        "SELECT f.uuid, fw.uuid, f.Rufname, f.Kurzname, f.aktiv
         FROM Fahrzeuge f JOIN Feuerwehr fw ON fw.id = f.FeuerwehrID ORDER BY f.Rufname",
    )?;
    for z in s.query_map([], |r| {
        Ok(FahrzeugJ {
            uuid: r.get(0)?,
            feuerwehr: r.get(1)?,
            rufname: r.get(2)?,
            kurzname: r.get(3)?,
            aktiv: r.get::<_, i64>(4)? == 1,
        })
    })? {
        fahrzeuge.push(z?);
    }

    let mut personal = Vec::new();
    let mut s = conn.prepare(
        "SELECT p.id, p.uuid, fw.uuid, p.Vorname, p.Nachname, p.Rang, p.aktiv
         FROM Personal p JOIN Feuerwehr fw ON fw.id = p.FeuerwehrID ORDER BY p.Nachname, p.Vorname",
    )?;
    let mut q = conn.prepare(
        "SELECT q.Kurzname FROM PersonalQualifikation pq JOIN Qualifikation q ON q.id = pq.QualifikationID
         WHERE pq.PersonalID = ?1 ORDER BY q.Kurzname",
    )?;
    let zeilen: Vec<(i64, PersonJ)> = s
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                PersonJ {
                    uuid: r.get(1)?,
                    feuerwehr: r.get(2)?,
                    vorname: r.get(3)?,
                    nachname: r.get(4)?,
                    rang: r.get(5)?,
                    aktiv: r.get::<_, i64>(6)? == 1,
                    qualifikationen: Vec::new(),
                },
            ))
        })?
        .collect::<Result<_>>()?;
    for (id, mut p) in zeilen {
        p.qualifikationen = q.query_map(params![id], |r| r.get(0))?.collect::<Result<_>>()?;
        personal.push(p);
    }

    let mut sprechgruppen = Vec::new();
    let mut s = conn.prepare("SELECT uuid, Name, Sortierung1, Sortierung2, Sortierung3 FROM Sprechgruppe ORDER BY Name")?;
    for z in s.query_map([], |r| {
        Ok(SprechgruppeJ {
            uuid: r.get(0)?,
            name: r.get(1)?,
            sortierung1: r.get(2)?,
            sortierung2: r.get(3)?,
            sortierung3: r.get(4)?,
        })
    })? {
        sprechgruppen.push(z?);
    }

    Ok(Paket {
        format: FORMAT.into(),
        version: VERSION,
        exportiert_am: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        feuerwehren,
        qualifikationen,
        fahrzeuge,
        personal,
        sprechgruppen,
    })
}

/// Schreibt das Paket als lesbar formatierte JSON-Datei.
pub fn schreiben(paket: &Paket, pfad: &std::path::Path) -> std::result::Result<(), String> {
    let text = serde_json::to_string_pretty(paket).map_err(|e| e.to_string())?;
    std::fs::write(pfad, text).map_err(|e| format!("Datei konnte nicht geschrieben werden: {e}"))
}

/// Liest eine Export-Datei und prüft Format und Version.
pub fn lesen(pfad: &std::path::Path) -> std::result::Result<Paket, String> {
    let text = std::fs::read_to_string(pfad).map_err(|e| format!("Datei konnte nicht gelesen werden: {e}"))?;
    let paket: Paket = serde_json::from_str(&text).map_err(|e| format!("Keine gültige Firecage-Stammdatendatei: {e}"))?;
    if paket.format != FORMAT {
        return Err(format!("Unbekanntes Dateiformat „{}“ (erwartet „{FORMAT}“).", paket.format));
    }
    if paket.version > VERSION {
        return Err(format!(
            "Die Datei hat Version {}, dieses Programm kennt nur bis Version {VERSION}. Bitte Firecage aktualisieren.",
            paket.version
        ));
    }
    Ok(paket)
}

// ───────────────────────────── Konfliktprüfung ─────────────────────────────

/// Datensatzart.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Art {
    Qualifikation,
    Feuerwehr,
    Fahrzeug,
    Person,
    Sprechgruppe,
}

impl Art {
    pub const ALLE: [Art; 5] = [Art::Feuerwehr, Art::Fahrzeug, Art::Person, Art::Qualifikation, Art::Sprechgruppe];
    pub fn name(self) -> &'static str {
        match self {
            Art::Qualifikation => "Qualifikationen",
            Art::Feuerwehr => "Feuerwehren",
            Art::Fahrzeug => "Fahrzeuge",
            Art::Person => "Personal",
            Art::Sprechgruppe => "Sprechgruppen",
        }
    }
}

/// Ergebnis des Vergleichs eines Datensatzes aus der Datei mit der Datenbank.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Gibt es lokal noch nicht → wird angelegt.
    Neu,
    /// Gibt es lokal genau so → nichts zu tun.
    Gleich,
    /// Gleiche UUID, aber anderer Inhalt (z. B. Name geändert).
    Konflikt,
    /// Andere UUID, aber gleicher Name → vermutlich doppelt.
    Duplikat,
    /// Kann nicht importiert werden (z. B. Feuerwehr fehlt).
    Fehler,
}

impl Status {
    pub const ALLE: [Status; 5] = [Status::Neu, Status::Gleich, Status::Konflikt, Status::Duplikat, Status::Fehler];
    pub fn name(self) -> &'static str {
        match self {
            Status::Neu => "neu",
            Status::Gleich => "unverändert",
            Status::Konflikt => "Konflikt",
            Status::Duplikat => "mögl. Duplikat",
            Status::Fehler => "Fehler",
        }
    }
}

/// Ein geprüfter Datensatz.
pub struct Eintrag {
    pub art: Art,
    /// Position in der entsprechenden Liste des Pakets.
    pub index: usize,
    pub bezeichnung: String,
    pub status: Status,
    /// Erklärung bei Konflikt/Duplikat/Fehler (welche Felder abweichen usw.).
    pub hinweis: String,
    /// id des passenden lokalen Datensatzes (bei Gleich/Konflikt/Duplikat).
    pub lokal_id: Option<i64>,
}

/// Ergebnis der Konfliktprüfung über alle Datensätze.
pub struct Analyse {
    pub eintraege: Vec<Eintrag>,
}

impl Analyse {
    /// Anzahl Einträge einer Art mit einem Status.
    pub fn anzahl(&self, art: Art, status: Status) -> usize {
        self.eintraege.iter().filter(|e| e.art == art && e.status == status).count()
    }
    /// Anzahl Einträge mit einem Status (alle Arten).
    pub fn summe(&self, status: Status) -> usize {
        self.eintraege.iter().filter(|e| e.status == status).count()
    }
}

/// Vergleichsschlüssel für Namen: ohne Groß/Klein und Leerzeichen am Rand.
fn norm(s: &str) -> String {
    s.trim().to_lowercase()
}

/// uuid → id einer Tabelle (nur feste Tabellennamen aus dem Code).
fn uuid_ids(conn: &Connection, tabelle: &str) -> Result<HashMap<String, i64>> {
    let mut s = conn.prepare(&format!("SELECT uuid, id FROM \"{tabelle}\""))?;
    let zeilen = s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    zeilen.collect()
}

/// Hängt "Feld: „alt“ → „neu“" an, wenn sich der Wert unterscheidet.
fn unterschied(liste: &mut Vec<String>, feld: &str, lokal: &str, datei: &str) {
    if lokal != datei {
        liste.push(format!("{feld}: „{lokal}“ → „{datei}“"));
    }
}

fn ja_nein(b: bool) -> &'static str {
    if b {
        "aktiv"
    } else {
        "inaktiv"
    }
}

/// Allgemeiner Vergleich: zuerst über die UUID, sonst über den Namensschlüssel.
/// Liefert Status, lokale id und (bei Treffer) den lokalen Datensatz.
fn vergleichen<'a, T>(
    datei: &T,
    lokal: &'a [T],
    ids: &HashMap<String, i64>,
    uuid: impl Fn(&T) -> &str,
    schluessel: impl Fn(&T) -> String,
    gleich: impl Fn(&T, &T) -> bool,
) -> (Status, Option<i64>, Option<&'a T>) {
    if let Some(l) = lokal.iter().find(|l| uuid(*l) == uuid(datei)) {
        let status = if gleich(l, datei) { Status::Gleich } else { Status::Konflikt };
        return (status, ids.get(uuid(l)).copied(), Some(l));
    }
    if let Some(l) = lokal.iter().find(|l| schluessel(*l) == schluessel(datei)) {
        return (Status::Duplikat, ids.get(uuid(l)).copied(), Some(l));
    }
    (Status::Neu, None, None)
}

/// Vergleicht jeden Datensatz der Datei mit der Datenbank.
pub fn analysieren(conn: &Connection, datei: &Paket) -> Result<Analyse> {
    let lokal = exportieren(conn)?;
    let mut eintraege = Vec::new();

    // Qualifikationen: Zuordnung über den Kurznamen
    let q_ids: HashMap<String, i64> = {
        let mut s = conn.prepare("SELECT Kurzname, id FROM Qualifikation")?;
        let z = s.query_map([], |r| Ok((norm(&r.get::<_, String>(0)?), r.get::<_, i64>(1)?)))?;
        z.collect::<Result<_>>()?
    };
    for (index, q) in datei.qualifikationen.iter().enumerate() {
        let (status, lokal_id, hinweis) = match lokal.qualifikationen.iter().find(|l| norm(&l.kurzname) == norm(&q.kurzname)) {
            Some(l) if l.vollname == q.vollname => (Status::Gleich, q_ids.get(&norm(&l.kurzname)).copied(), String::new()),
            Some(l) => (
                Status::Konflikt,
                q_ids.get(&norm(&l.kurzname)).copied(),
                format!("Bezeichnung: „{}“ → „{}“", l.vollname, q.vollname),
            ),
            None => (Status::Neu, None, String::new()),
        };
        eintraege.push(Eintrag {
            art: Art::Qualifikation,
            index,
            bezeichnung: format!("{} – {}", q.kurzname, q.vollname),
            status,
            hinweis,
            lokal_id,
        });
    }

    // Feuerwehren: Schlüssel = Name
    let fw_ids = uuid_ids(conn, "Feuerwehr")?;
    for (index, f) in datei.feuerwehren.iter().enumerate() {
        let (status, lokal_id, l) = vergleichen(
            f,
            &lokal.feuerwehren,
            &fw_ids,
            |x| x.uuid.as_str(),
            |x| norm(&x.name),
            |a, b| a.name == b.name && a.kurzname == b.kurzname && a.aktiv == b.aktiv,
        );
        let mut d = Vec::new();
        if let Some(l) = l {
            unterschied(&mut d, "Name", &l.name, &f.name);
            unterschied(
                &mut d,
                "Kurzname",
                l.kurzname.as_deref().unwrap_or(""),
                f.kurzname.as_deref().unwrap_or(""),
            );
            unterschied(&mut d, "Status", ja_nein(l.aktiv), ja_nein(f.aktiv));
        }
        let hinweis = match status {
            Status::Duplikat => format!("Eine Feuerwehr „{}“ gibt es lokal bereits (andere Kennung).", f.name),
            _ => d.join(", "),
        };
        eintraege.push(Eintrag {
            art: Art::Feuerwehr,
            index,
            bezeichnung: f.name.clone(),
            status,
            hinweis,
            lokal_id,
        });
    }

    // Prüft, ob eine Feuerwehr-UUID in der Datei oder lokal bekannt ist.
    let fw_bekannt = |uuid: &str| datei.feuerwehren.iter().any(|f| f.uuid == uuid) || fw_ids.contains_key(uuid);

    // Fahrzeuge: Schlüssel = Rufname
    let fz_ids = uuid_ids(conn, "Fahrzeuge")?;
    for (index, f) in datei.fahrzeuge.iter().enumerate() {
        let (mut status, lokal_id, l) = vergleichen(
            f,
            &lokal.fahrzeuge,
            &fz_ids,
            |x| x.uuid.as_str(),
            |x| norm(&x.rufname),
            |a, b| a.feuerwehr == b.feuerwehr && a.rufname == b.rufname && a.kurzname == b.kurzname && a.aktiv == b.aktiv,
        );
        let mut d = Vec::new();
        if let Some(l) = l {
            unterschied(&mut d, "Rufname", &l.rufname, &f.rufname);
            unterschied(&mut d, "Kurzname", &l.kurzname, &f.kurzname);
            unterschied(&mut d, "Status", ja_nein(l.aktiv), ja_nein(f.aktiv));
            if l.feuerwehr != f.feuerwehr {
                d.push("andere Feuerwehr".into());
            }
        }
        let mut hinweis = match status {
            Status::Duplikat => format!("Ein Fahrzeug mit Rufname „{}“ gibt es lokal bereits (andere Kennung).", f.rufname),
            _ => d.join(", "),
        };
        if !fw_bekannt(f.feuerwehr.as_str()) {
            status = Status::Fehler;
            hinweis = "Zugehörige Feuerwehr fehlt in Datei und Datenbank.".into();
        }
        eintraege.push(Eintrag {
            art: Art::Fahrzeug,
            index,
            bezeichnung: format!("{} ({})", f.rufname, f.kurzname),
            status,
            hinweis,
            lokal_id,
        });
    }

    // Personal: Schlüssel = Vorname + Nachname
    let p_ids = uuid_ids(conn, "Personal")?;
    let sortiert = |v: &[String]| {
        let mut v: Vec<String> = v.iter().map(|s| norm(s)).collect();
        v.sort();
        v
    };
    for (index, p) in datei.personal.iter().enumerate() {
        let (mut status, lokal_id, l) = vergleichen(
            p,
            &lokal.personal,
            &p_ids,
            |x| x.uuid.as_str(),
            |x| format!("{} {}", norm(&x.vorname), norm(&x.nachname)),
            |a, b| {
                a.feuerwehr == b.feuerwehr
                    && a.vorname == b.vorname
                    && a.nachname == b.nachname
                    && a.rang == b.rang
                    && a.aktiv == b.aktiv
                    && sortiert(a.qualifikationen.as_slice()) == sortiert(b.qualifikationen.as_slice())
            },
        );
        let mut d = Vec::new();
        if let Some(l) = l {
            unterschied(&mut d, "Vorname", &l.vorname, &p.vorname);
            unterschied(&mut d, "Nachname", &l.nachname, &p.nachname);
            unterschied(&mut d, "Rang", &l.rang, &p.rang);
            unterschied(&mut d, "Status", ja_nein(l.aktiv), ja_nein(p.aktiv));
            unterschied(
                &mut d,
                "Qualifikationen",
                &l.qualifikationen.join(", "),
                &p.qualifikationen.join(", "),
            );
            if l.feuerwehr != p.feuerwehr {
                d.push("andere Feuerwehr".into());
            }
        }
        let mut hinweis = match status {
            Status::Duplikat => format!("„{} {}“ gibt es lokal bereits (andere Kennung).", p.vorname, p.nachname),
            _ => d.join(", "),
        };
        // Qualifikationen, die weder in der Datei noch lokal existieren, werden ausgelassen.
        let unbekannt: Vec<&str> = p
            .qualifikationen
            .iter()
            .filter(|k| !datei.qualifikationen.iter().any(|q| norm(&q.kurzname) == norm(k)) && !q_ids.contains_key(&norm(k)))
            .map(|k| k.as_str())
            .collect();
        if !unbekannt.is_empty() {
            if !hinweis.is_empty() {
                hinweis.push_str(" · ");
            }
            hinweis.push_str(&format!(
                "unbekannte Qualifikation(en) werden ausgelassen: {}",
                unbekannt.join(", ")
            ));
        }
        if !fw_bekannt(p.feuerwehr.as_str()) {
            status = Status::Fehler;
            hinweis = "Zugehörige Feuerwehr fehlt in Datei und Datenbank.".into();
        }
        eintraege.push(Eintrag {
            art: Art::Person,
            index,
            bezeichnung: format!("{} {} ({})", p.vorname, p.nachname, p.rang),
            status,
            hinweis,
            lokal_id,
        });
    }

    // Sprechgruppen: Schlüssel = Name
    let sg_ids = uuid_ids(conn, "Sprechgruppe")?;
    for (index, s) in datei.sprechgruppen.iter().enumerate() {
        let (status, lokal_id, l) = vergleichen(
            s,
            &lokal.sprechgruppen,
            &sg_ids,
            |x| x.uuid.as_str(),
            |x| norm(&x.name),
            |a, b| a.name == b.name && a.sortierung1 == b.sortierung1 && a.sortierung2 == b.sortierung2 && a.sortierung3 == b.sortierung3,
        );
        let mut d = Vec::new();
        if let Some(l) = l {
            unterschied(&mut d, "Name", &l.name, &s.name);
            for (feld, a, b) in [
                ("Sortierung1", &l.sortierung1, &s.sortierung1),
                ("Sortierung2", &l.sortierung2, &s.sortierung2),
                ("Sortierung3", &l.sortierung3, &s.sortierung3),
            ] {
                unterschied(&mut d, feld, a.as_deref().unwrap_or(""), b.as_deref().unwrap_or(""));
            }
        }
        let hinweis = match status {
            Status::Duplikat => format!("Eine Sprechgruppe „{}“ gibt es lokal bereits (andere Kennung).", s.name),
            _ => d.join(", "),
        };
        eintraege.push(Eintrag {
            art: Art::Sprechgruppe,
            index,
            bezeichnung: s.name.clone(),
            status,
            hinweis,
            lokal_id,
        });
    }

    Ok(Analyse { eintraege })
}

// ───────────────────────────── Import ─────────────────────────────

/// Umgang mit möglichen Duplikaten (gleicher Name, andere Kennung).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum DuplikatModus {
    /// Lokalen Datensatz behalten, Datei-Eintrag ignorieren.
    #[default]
    Ueberspringen,
    /// Lokalen Datensatz mit den Werten aus der Datei aktualisieren.
    Zusammenfuehren,
    /// Trotzdem als eigenen Datensatz anlegen.
    NeuAnlegen,
}

/// Vom Benutzer gewählte Import-Optionen.
#[derive(Clone, Copy, Default)]
pub struct Optionen {
    /// Bei Konflikten (gleiche Kennung, anderer Inhalt) die Werte aus der Datei übernehmen.
    pub konflikt_ueberschreiben: bool,
    pub duplikat: DuplikatModus,
}

/// Zusammenfassung nach dem Import.
#[derive(Default)]
pub struct Bericht {
    pub neu: usize,
    pub aktualisiert: usize,
    pub unveraendert: usize,
    pub uebersprungen: usize,
    pub fehler: usize,
}

impl Bericht {
    pub fn text(&self) -> String {
        format!(
            "Import abgeschlossen: {} neu, {} aktualisiert, {} unverändert, {} übersprungen, {} Fehler.",
            self.neu, self.aktualisiert, self.unveraendert, self.uebersprungen, self.fehler
        )
    }
}

/// Was mit einem Datensatz geschieht.
enum Tun {
    Anlegen,
    Aktualisieren(i64),
    /// Nichts tun; ggf. lokale id für Verweise merken.
    Behalten(Option<i64>),
}

fn entscheiden(e: &Eintrag, opt: &Optionen, b: &mut Bericht) -> Tun {
    match (e.status, e.lokal_id) {
        (Status::Neu, _) => Tun::Anlegen,
        (Status::Gleich, id) => {
            b.unveraendert += 1;
            Tun::Behalten(id)
        }
        (Status::Konflikt, Some(id)) if opt.konflikt_ueberschreiben => Tun::Aktualisieren(id),
        (Status::Duplikat, Some(id)) if opt.duplikat == DuplikatModus::Zusammenfuehren => Tun::Aktualisieren(id),
        (Status::Duplikat, _) if opt.duplikat == DuplikatModus::NeuAnlegen => Tun::Anlegen,
        (Status::Fehler, _) => {
            b.fehler += 1;
            Tun::Behalten(None)
        }
        (_, id) => {
            b.uebersprungen += 1;
            Tun::Behalten(id)
        }
    }
}

/// Übernimmt die Datei in die Datenbank – alles in einer Transaktion:
/// tritt ein Fehler auf, bleibt die Datenbank unverändert.
/// Die Konfliktprüfung wird dabei neu durchgeführt (aktueller Datenbankstand).
pub fn importieren(conn: &Connection, datei: &Paket, opt: &Optionen) -> Result<Bericht> {
    let tx = conn.unchecked_transaction()?;
    let analyse = analysieren(&tx, datei)?;
    let mut b = Bericht::default();
    let eintraege = |art: Art| analyse.eintraege.iter().filter(move |e| e.art == art);

    // 1) Qualifikationen
    for e in eintraege(Art::Qualifikation) {
        let q = &datei.qualifikationen[e.index];
        match entscheiden(e, opt, &mut b) {
            Tun::Anlegen => {
                // Datei-UUID übernehmen, falls lokal noch frei – sonst neue vergeben
                let frei: bool = tx.query_row("SELECT COUNT(*) = 0 FROM Qualifikation WHERE uuid = ?1", params![q.uuid], |r| {
                    r.get(0)
                })?;
                let uuid = if frei { q.uuid.clone() } else { uuid::Uuid::new_v4().to_string() };
                tx.execute(
                    "INSERT INTO Qualifikation (uuid, Kurzname, Vollname) VALUES (?1, ?2, ?3)",
                    params![uuid, q.kurzname, q.vollname],
                )?;
                b.neu += 1;
            }
            Tun::Aktualisieren(id) => {
                tx.execute("UPDATE Qualifikation SET Vollname = ?1 WHERE id = ?2", params![q.vollname, id])?;
                b.aktualisiert += 1;
            }
            Tun::Behalten(_) => {}
        }
    }
    let q_ids: HashMap<String, i64> = {
        let mut s = tx.prepare("SELECT Kurzname, id FROM Qualifikation")?;
        let z = s.query_map([], |r| Ok((norm(&r.get::<_, String>(0)?), r.get::<_, i64>(1)?)))?;
        z.collect::<Result<_>>()?
    };

    // 2) Feuerwehren – Datei-UUID → lokale id merken (für Fahrzeuge/Personal)
    let mut fw_map: HashMap<String, i64> = uuid_ids(&tx, "Feuerwehr")?;
    for e in eintraege(Art::Feuerwehr) {
        let f = &datei.feuerwehren[e.index];
        let id = match entscheiden(e, opt, &mut b) {
            Tun::Anlegen => {
                tx.execute(
                    "INSERT INTO Feuerwehr (uuid, aktiv, Name, Kurzname) VALUES (?1, ?2, ?3, ?4)",
                    params![f.uuid, f.aktiv as i64, f.name, f.kurzname],
                )?;
                b.neu += 1;
                Some(tx.last_insert_rowid())
            }
            Tun::Aktualisieren(id) => {
                tx.execute(
                    "UPDATE Feuerwehr SET Name = ?1, Kurzname = ?2, aktiv = ?3 WHERE id = ?4",
                    params![f.name, f.kurzname, f.aktiv as i64, id],
                )?;
                b.aktualisiert += 1;
                Some(id)
            }
            Tun::Behalten(id) => id,
        };
        if let Some(id) = id {
            fw_map.insert(f.uuid.clone(), id);
        }
    }

    // 3) Sprechgruppen
    for e in eintraege(Art::Sprechgruppe) {
        let s = &datei.sprechgruppen[e.index];
        match entscheiden(e, opt, &mut b) {
            Tun::Anlegen => {
                tx.execute(
                    "INSERT INTO Sprechgruppe (uuid, Name, Sortierung1, Sortierung2, Sortierung3) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![s.uuid, s.name, s.sortierung1, s.sortierung2, s.sortierung3],
                )?;
                b.neu += 1;
            }
            Tun::Aktualisieren(id) => {
                tx.execute(
                    "UPDATE Sprechgruppe SET Name = ?1, Sortierung1 = ?2, Sortierung2 = ?3, Sortierung3 = ?4 WHERE id = ?5",
                    params![s.name, s.sortierung1, s.sortierung2, s.sortierung3, id],
                )?;
                b.aktualisiert += 1;
            }
            Tun::Behalten(_) => {}
        }
    }

    // 4) Fahrzeuge
    for e in eintraege(Art::Fahrzeug) {
        let f = &datei.fahrzeuge[e.index];
        let tun = entscheiden(e, opt, &mut b);
        let Some(&fw_id) = fw_map.get(&f.feuerwehr) else {
            // Feuerwehr wurde nicht übernommen → Fahrzeug kann nicht zugeordnet werden
            if !matches!(tun, Tun::Behalten(_)) {
                b.fehler += 1;
            }
            continue;
        };
        match tun {
            Tun::Anlegen => {
                tx.execute(
                    "INSERT INTO Fahrzeuge (uuid, FeuerwehrID, Rufname, Kurzname, aktiv) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![f.uuid, fw_id, f.rufname, f.kurzname, f.aktiv as i64],
                )?;
                b.neu += 1;
            }
            Tun::Aktualisieren(id) => {
                tx.execute(
                    "UPDATE Fahrzeuge SET FeuerwehrID = ?1, Rufname = ?2, Kurzname = ?3, aktiv = ?4 WHERE id = ?5",
                    params![fw_id, f.rufname, f.kurzname, f.aktiv as i64, id],
                )?;
                b.aktualisiert += 1;
            }
            Tun::Behalten(_) => {}
        }
    }

    // 5) Personal inkl. Qualifikationen
    for e in eintraege(Art::Person) {
        let p = &datei.personal[e.index];
        let tun = entscheiden(e, opt, &mut b);
        let Some(&fw_id) = fw_map.get(&p.feuerwehr) else {
            if !matches!(tun, Tun::Behalten(_)) {
                b.fehler += 1;
            }
            continue;
        };
        let person_id = match tun {
            Tun::Anlegen => {
                tx.execute(
                    "INSERT INTO Personal (uuid, FeuerwehrID, aktiv, Vorname, Nachname, Rang) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![p.uuid, fw_id, p.aktiv as i64, p.vorname, p.nachname, p.rang],
                )?;
                b.neu += 1;
                tx.last_insert_rowid()
            }
            Tun::Aktualisieren(id) => {
                tx.execute(
                    "UPDATE Personal SET FeuerwehrID = ?1, aktiv = ?2, Vorname = ?3, Nachname = ?4, Rang = ?5 WHERE id = ?6",
                    params![fw_id, p.aktiv as i64, p.vorname, p.nachname, p.rang, id],
                )?;
                b.aktualisiert += 1;
                id
            }
            Tun::Behalten(_) => continue,
        };
        // Qualifikationen der Person auf den Stand der Datei setzen
        tx.execute("DELETE FROM PersonalQualifikation WHERE PersonalID = ?1", params![person_id])?;
        for k in &p.qualifikationen {
            if let Some(q_id) = q_ids.get(&norm(k)) {
                tx.execute(
                    "INSERT OR IGNORE INTO PersonalQualifikation (PersonalID, QualifikationID) VALUES (?1, ?2)",
                    params![person_id, q_id],
                )?;
            }
        }
    }

    tx.commit()?;
    Ok(b)
}
