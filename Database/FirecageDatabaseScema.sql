CREATE TABLE IF NOT EXISTS "Fahrzeuge" (
	"id" INTEGER NOT NULL,
	"uuid" TEXT NOT NULL UNIQUE,
	"FeuerwehrID" INTEGER NOT NULL,
	"Rufname" TEXT NOT NULL,
	"Kurzname" TEXT NOT NULL,
	"aktiv" INTEGER NOT NULL CHECK(aktiv IN(0,1)),
	PRIMARY KEY("id"),
	FOREIGN KEY ("FeuerwehrID") REFERENCES "Feuerwehr"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION
);

CREATE TABLE IF NOT EXISTS "Personal" (
	"id" INTEGER NOT NULL,
	"uuid" TEXT NOT NULL UNIQUE,
	"FeuerwehrID" INTEGER NOT NULL,
	"aktiv" INTEGER NOT NULL CHECK(aktiv IN(0,1)),
	"Vorname" TEXT NOT NULL,
	"Nachname" TEXT NOT NULL,
	"Rang" TEXT NOT NULL,
	PRIMARY KEY("id"),
	FOREIGN KEY ("FeuerwehrID") REFERENCES "Feuerwehr"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION
);

CREATE TABLE IF NOT EXISTS "Feuerwehr" (
	"id" INTEGER NOT NULL,
	"uuid" TEXT NOT NULL UNIQUE,
	"aktiv" INTEGER NOT NULL CHECK(aktiv IN(0,1)),
	"Name" TEXT NOT NULL,
	"Kurzname" TEXT,
	PRIMARY KEY("id")
);

CREATE TABLE IF NOT EXISTS "Funktagebuch" (
	"id" INTEGER NOT NULL,
	"EinsatzID" INTEGER NOT NULL,
	"EingegangenUm" INTEGER NOT NULL,
	"Inhalt" TEXT NOT NULL,
	"SprechgruppeID" INTEGER NOT NULL,
	"Rufstelle" TEXT NOT NULL,
	"Meldestelle" TEXT NOT NULL,
	PRIMARY KEY("id"),
	FOREIGN KEY ("EinsatzID") REFERENCES "Einsatz"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION,
	FOREIGN KEY ("SprechgruppeID") REFERENCES "Sprechgruppe"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION
);

CREATE TABLE IF NOT EXISTS "Aufgaben" (
	"id" INTEGER NOT NULL,
	"EinsatzID" INTEGER NOT NULL,
	"AbschnittID" INTEGER,
	"FahrzeugID" INTEGER,
	"Zeitpunkt" INTEGER NOT NULL,
	-- Zeigt an ob Aufgabe erledigt wurde
	"Erledigt" INTEGER NOT NULL CHECK(Erledigt IN(0,1)),
	-- Zeit bis Rückgefragt werden sollte
	"NachfragenUm" INTEGER,
	"EinsatzortID" INTEGER,
	"Beschreibung" TEXT NOT NULL,
	"KategorieID" INTEGER NOT NULL,
	"AtemschutzeinsatzID" INTEGER,
	PRIMARY KEY("id"),
	FOREIGN KEY ("EinsatzID") REFERENCES "Einsatz"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION,
	FOREIGN KEY ("FahrzeugID") REFERENCES "Fahrzeuge"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION,
	FOREIGN KEY ("AbschnittID") REFERENCES "Einsatzabschnitt"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION,
	FOREIGN KEY ("EinsatzortID") REFERENCES "Einsatzort"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION,
	FOREIGN KEY ("KategorieID") REFERENCES "AufgabeKategorie"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION,
	FOREIGN KEY ("AtemschutzeinsatzID") REFERENCES "Atemschutzeinsatz"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION
);

CREATE TABLE IF NOT EXISTS "Einsatz" (
	"id" INTEGER NOT NULL,
	"uuid" TEXT NOT NULL UNIQUE,
	"EinsatztypID" INTEGER NOT NULL,
	"Einsatzbeginn" INTEGER NOT NULL,
	"Einsatzende" INTEGER,
	"Stichwort" TEXT NOT NULL,
	PRIMARY KEY("id"),
	FOREIGN KEY ("EinsatztypID") REFERENCES "Einsatztyp"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION
);

CREATE TABLE IF NOT EXISTS "Einsatzabschnitt" (
	"id" INTEGER NOT NULL,
	"EinsatzID" INTEGER NOT NULL,
	"Name" TEXT NOT NULL,
	PRIMARY KEY("id"),
	FOREIGN KEY ("EinsatzID") REFERENCES "Einsatz"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION
);

CREATE TABLE IF NOT EXISTS "Meldetagebuch" (
	"id" INTEGER NOT NULL,
	"EinsatzID" INTEGER NOT NULL,
	-- Typ der Meldung
	"KategorieID" INTEGER NOT NULL,
	"Inhalt" TEXT NOT NULL,
	-- Zeitpunkt der Meldung
	"EingegangenUm" INTEGER NOT NULL,
	"Von" TEXT NOT NULL,
	"An" TEXT NOT NULL,
	PRIMARY KEY("id"),
	FOREIGN KEY ("EinsatzID") REFERENCES "Einsatz"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION,
	FOREIGN KEY ("KategorieID") REFERENCES "MeldungKategorie"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION
);

CREATE TABLE IF NOT EXISTS "Adresse" (
	"id" INTEGER NOT NULL,
	"uuid" TEXT NOT NULL UNIQUE,
	"Ortschaft" TEXT NOT NULL,
	"Strasse" TEXT NOT NULL,
	"Hausnummer" TEXT NOT NULL,
	"Stiege" TEXT,
	PRIMARY KEY("id")
);

CREATE TABLE IF NOT EXISTS "Einsatzort" (
	"id" INTEGER NOT NULL,
	"Beschreibung" TEXT NOT NULL,
	"Laengengrad" REAL,
	"Breitengrad" REAL,
	"AdresseID" INTEGER,
	"EinsatzID" INTEGER,
	PRIMARY KEY("id"),
	FOREIGN KEY ("AdresseID") REFERENCES "Adresse"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION,
	FOREIGN KEY ("EinsatzID") REFERENCES "Einsatz"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION
);

CREATE TABLE IF NOT EXISTS "Qualifikation" (
	"id" INTEGER NOT NULL,
	"uuid" TEXT NOT NULL UNIQUE,
	"Kurzname" TEXT NOT NULL,
	"Vollname" TEXT NOT NULL,
	PRIMARY KEY("id")
);

CREATE TABLE IF NOT EXISTS "PersonalQualifikation" (
	"id" INTEGER NOT NULL,
	"PersonalID" INTEGER NOT NULL,
	"QualifikationID" INTEGER NOT NULL,
	PRIMARY KEY("id"),
	FOREIGN KEY ("PersonalID") REFERENCES "Personal"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION,
	FOREIGN KEY ("QualifikationID") REFERENCES "Qualifikation"("id")
	ON UPDATE NO ACTION ON DELETE NO ACTION,
	CONSTRAINT "PersonalQualifikation_unique_0" UNIQUE ("PersonalID", "QualifikationID")
);

CREATE TABLE IF NOT EXISTS "Sprechgruppe" (
	"id" INTEGER NOT NULL,
	"uuid" TEXT NOT NULL UNIQUE,
	"Name" TEXT NOT NULL,
	"Sortierung1" TEXT,
	"Sortierung2" TEXT,
	"Sortierung3" TEXT,
	PRIMARY KEY("id")
);

CREATE TABLE IF NOT EXISTS "AufgabeKategorie" (
	"id" INTEGER NOT NULL,
	"Name" TEXT NOT NULL,
	"Farbe" TEXT,
	"Sortierung" INTEGER NOT NULL,
	"aktiv" INTEGER NOT NULL CHECK(aktiv IN(0,1)),
	PRIMARY KEY("id")
);

CREATE TABLE IF NOT EXISTS "MeldungKategorie" (
	"id" INTEGER NOT NULL,
	"Name" TEXT NOT NULL,
	PRIMARY KEY("id")
);

CREATE TABLE IF NOT EXISTS "Einsatztyp" (
	"id" INTEGER NOT NULL,
	"Name" TEXT NOT NULL,
	PRIMARY KEY("id")
);

CREATE TABLE IF NOT EXISTS "Atemschutzeinsatz" (
	"id" INTEGER NOT NULL,
	"ATSTruppID" INTEGER NOT NULL,
	"EinsatzID" INTEGER,
	-- in sekunden
	"Einsatzzeit" INTEGER,
	"EinsatzStart" INTEGER,
	"AbgeschlossenUm" INTEGER,
	"Erledigt" INTEGER CHECK(Erledigt IN(0,1)),
	"ATFName" TEXT,
	"DruckATFStart" INTEGER,
	"DruckATFNachfrage" INTEGER,
	"DruckATFEnde" INTEGER,
	"ATMName" TEXT,
	"DruckATMStart" INTEGER,
	"DruckATMNachfrage" INTEGER,
	"DruckATMEnde" INTEGER,
	"WTFName" TEXT,
	"DruckWTFStart" INTEGER,
	"DruckWTFNachfrage" INTEGER,
	"DruckWTFEnde" INTEGER,
	PRIMARY KEY("id")
);
