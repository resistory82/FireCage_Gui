//! Konfigurationsbildschirme für die Stammdaten:
//! Feuerwehr, Fahrzeugpark, Personal und Sprechgruppen.
//!
//! Aufbau jeweils gleich: oben ein Formular zum Anlegen, darunter die Liste.
//! Einträge werden nicht gelöscht, sondern über "aktiv" ausgeblendet,
//! damit alte Einsätze vollständig dokumentiert bleiben.

use super::{auswahl_feld, optional};
use crate::App;
use eframe::egui;

/// Feuerwehren anlegen und aktivieren/deaktivieren.
pub fn feuerwehr(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Feuerwehr");
    ui.add_space(10.0);

    // Formular: neue Feuerwehr
    ui.horizontal(|ui| {
        ui.label("Name");
        ui.add(egui::TextEdit::singleline(&mut app.f.fw_name).desired_width(260.0));
        ui.label("Kurzname");
        ui.add(egui::TextEdit::singleline(&mut app.f.fw_kurz).desired_width(120.0));
        if ui.button("➕ Anlegen").clicked() {
            if app.f.fw_name.trim().is_empty() {
                app.fehler("Bitte einen Namen eingeben.");
            } else {
                let r = app.db.feuerwehr_anlegen(app.f.fw_name.trim(), optional(&app.f.fw_kurz));
                if app.ok(r).is_some() {
                    app.info(format!("Feuerwehr „{}“ angelegt.", app.f.fw_name.trim()));
                    app.f.fw_name.clear();
                    app.f.fw_kurz.clear();
                }
            }
        }
    });
    ui.separator();

    // Liste aller Feuerwehren
    let liste = app.ok(app.db.feuerwehren()).unwrap_or_default();
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("fw_liste")
            .striped(true)
            .num_columns(3)
            .spacing([30.0, 8.0])
            .show(ui, |ui| {
                ui.strong("Name");
                ui.strong("Kurzname");
                ui.strong("Aktiv");
                ui.end_row();
                for fw in &liste {
                    ui.label(fw.name.as_str());
                    ui.label(fw.kurzname.as_deref().unwrap_or("–"));
                    let mut aktiv = fw.aktiv;
                    if ui.checkbox(&mut aktiv, "").changed() {
                        let r = app.db.aktiv_setzen("Feuerwehr", fw.id, aktiv);
                        app.ok(r);
                    }
                    ui.end_row();
                }
            });
    });
}

/// Fahrzeuge anlegen (einer Feuerwehr zugeordnet) und aktivieren/deaktivieren.
pub fn fahrzeugpark(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Fahrzeugpark");
    ui.add_space(10.0);

    let feuerwehren = app.ok(app.db.feuerwehr_auswahl()).unwrap_or_default();
    if feuerwehren.is_empty() {
        ui.label("Bitte zuerst unter „Feuerwehr“ eine Feuerwehr anlegen.");
        return;
    }

    // Formular: neues Fahrzeug
    ui.horizontal(|ui| {
        ui.label("Feuerwehr");
        auswahl_feld(ui, "fz_fw", &mut app.f.fz_feuerwehr, &feuerwehren, None);
        ui.label("Rufname");
        ui.add(
            egui::TextEdit::singleline(&mut app.f.fz_rufname)
                .desired_width(200.0)
                .hint_text("z. B. Tank Musterdorf"),
        );
        ui.label("Kurzname");
        ui.add(
            egui::TextEdit::singleline(&mut app.f.fz_kurz)
                .desired_width(100.0)
                .hint_text("z. B. TLF"),
        );
        if ui.button("➕ Anlegen").clicked() {
            match app.f.fz_feuerwehr {
                None => app.fehler("Bitte eine Feuerwehr wählen."),
                Some(_) if app.f.fz_rufname.trim().is_empty() || app.f.fz_kurz.trim().is_empty() => {
                    app.fehler("Bitte Rufname und Kurzname eingeben.")
                }
                Some(fw) => {
                    let r = app.db.fahrzeug_anlegen(fw, app.f.fz_rufname.trim(), app.f.fz_kurz.trim());
                    if app.ok(r).is_some() {
                        app.info(format!("Fahrzeug „{}“ angelegt.", app.f.fz_rufname.trim()));
                        app.f.fz_rufname.clear();
                        app.f.fz_kurz.clear();
                    }
                }
            }
        }
    });
    ui.separator();

    // Liste aller Fahrzeuge
    let liste = app.ok(app.db.fahrzeuge()).unwrap_or_default();
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("fz_liste")
            .striped(true)
            .num_columns(4)
            .spacing([30.0, 8.0])
            .show(ui, |ui| {
                for kopf in ["Feuerwehr", "Rufname", "Kurzname", "Aktiv"] {
                    ui.strong(kopf);
                }
                ui.end_row();
                for fz in &liste {
                    ui.label(fz.feuerwehr.as_str());
                    ui.label(fz.rufname.as_str());
                    ui.label(fz.kurzname.as_str());
                    let mut aktiv = fz.aktiv;
                    if ui.checkbox(&mut aktiv, "").changed() {
                        let r = app.db.aktiv_setzen("Fahrzeuge", fz.id, aktiv);
                        app.ok(r);
                    }
                    ui.end_row();
                }
            });
    });
}

/// Personal anlegen (mit Qualifikationen als Häkchen) und aktivieren/deaktivieren.
pub fn personal(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Personal");
    ui.add_space(10.0);

    let feuerwehren = app.ok(app.db.feuerwehr_auswahl()).unwrap_or_default();
    if feuerwehren.is_empty() {
        ui.label("Bitte zuerst unter „Feuerwehr“ eine Feuerwehr anlegen.");
        return;
    }
    let qualis = app.ok(app.db.qualifikationen()).unwrap_or_default();

    // Formular: neue Person
    egui::Grid::new("p_form").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
        ui.label("Feuerwehr");
        auswahl_feld(ui, "p_fw", &mut app.f.p_feuerwehr, &feuerwehren, None);
        ui.end_row();
        ui.label("Vorname");
        ui.add(egui::TextEdit::singleline(&mut app.f.p_vorname).desired_width(220.0));
        ui.end_row();
        ui.label("Nachname");
        ui.add(egui::TextEdit::singleline(&mut app.f.p_nachname).desired_width(220.0));
        ui.end_row();
        ui.label("Rang");
        ui.add(
            egui::TextEdit::singleline(&mut app.f.p_rang)
                .desired_width(220.0)
                .hint_text("z. B. OFM"),
        );
        ui.end_row();
        ui.label("Qualifikationen");
        ui.horizontal_wrapped(|ui| {
            for q in &qualis {
                let mut gewaehlt = app.f.p_qualis.contains(&q.id);
                if ui.checkbox(&mut gewaehlt, q.name.as_str()).changed() {
                    if gewaehlt {
                        app.f.p_qualis.push(q.id);
                    } else {
                        app.f.p_qualis.retain(|x| *x != q.id);
                    }
                }
            }
        });
        ui.end_row();
    });
    if ui.button("➕ Person anlegen").clicked() {
        let f = &app.f;
        match f.p_feuerwehr {
            None => app.fehler("Bitte eine Feuerwehr wählen."),
            Some(_) if f.p_vorname.trim().is_empty() || f.p_nachname.trim().is_empty() || f.p_rang.trim().is_empty() => {
                app.fehler("Bitte Vorname, Nachname und Rang eingeben.")
            }
            Some(fw) => {
                let r = app
                    .db
                    .person_anlegen(fw, f.p_vorname.trim(), f.p_nachname.trim(), f.p_rang.trim(), &f.p_qualis);
                if app.ok(r).is_some() {
                    app.info(format!("{} {} angelegt.", app.f.p_vorname.trim(), app.f.p_nachname.trim()));
                    app.f.p_vorname.clear();
                    app.f.p_nachname.clear();
                    app.f.p_rang.clear();
                    app.f.p_qualis.clear();
                }
            }
        }
    }
    ui.separator();

    // Liste des Personalstands
    let liste = app.ok(app.db.personal()).unwrap_or_default();
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("p_liste")
            .striped(true)
            .num_columns(6)
            .spacing([30.0, 8.0])
            .show(ui, |ui| {
                for kopf in ["Nachname", "Vorname", "Rang", "Feuerwehr", "Qualifikationen", "Aktiv"] {
                    ui.strong(kopf);
                }
                ui.end_row();
                for p in &liste {
                    ui.label(p.nachname.as_str());
                    ui.label(p.vorname.as_str());
                    ui.label(p.rang.as_str());
                    ui.label(p.feuerwehr.as_str());
                    ui.label(p.qualifikationen.as_str());
                    let mut aktiv = p.aktiv;
                    if ui.checkbox(&mut aktiv, "").changed() {
                        let r = app.db.aktiv_setzen("Personal", p.id, aktiv);
                        app.ok(r);
                    }
                    ui.end_row();
                }
            });
    });
}

/// Sprechgruppen anlegen (werden im Funktagebuch benötigt).
pub fn sprechgruppen(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Sprechgruppen");
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        ui.label("Name");
        let feld = ui.add(
            egui::TextEdit::singleline(&mut app.f.sg_name)
                .desired_width(260.0)
                .hint_text("z. B. BFKDO-01"),
        );
        let eintragen = ui.button("➕ Anlegen").clicked() || super::enter_gedrueckt(&feld, ui);
        if eintragen {
            if app.f.sg_name.trim().is_empty() {
                app.fehler("Bitte einen Namen eingeben.");
            } else {
                let r = app.db.sprechgruppe_anlegen(app.f.sg_name.trim());
                if app.ok(r).is_some() {
                    app.f.sg_name.clear();
                }
            }
        }
    });
    ui.separator();

    let liste = app.ok(app.db.sprechgruppen()).unwrap_or_default();
    for sg in &liste {
        ui.label(sg.name.as_str());
    }
}
