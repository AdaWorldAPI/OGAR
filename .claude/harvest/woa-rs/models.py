"""WoA sink-in substrate v2 — generated, do not hand-edit.

Sources: /home/user/WoA/models.py + /home/user/WoA/woa/models_shop.py (READ-ONLY corpus, 151 `db.Model` classes harvested)
Pipeline: ruff_sqlalchemy_spo::extract_file (x2, .models.extend() merge)
          -> ogar-from-ruff::lift_model_graph_sqlalchemy
          -> ogar-from-ruff::mint::compile_graph_sqlalchemy::<WoaPort>
          -> ogar-from-ruff::emit::emit_python (+ emit_python_prelude)
Metrics: 151 classes, 2154 attributes, 112 associations, 6 aliased (WOA_ALIASES convergence pin) / 145 bootstrap (classid 0x0000_0003).
Dangling .spo edges: 0/112.
TimesheetActivity -> classid 0x01030003 (concept 0x0103, app 0x0003)
"""

from __future__ import annotations
from dataclasses import dataclass
from typing import ClassVar, Optional
from ogar_runtime import (
    OgScalar, OgStr, OgInt, OgFloat, OgMoney, OgBool,
    OgDate, OgDateTime, OgBytes, OgSelection, OgJson,
    ToOne, ToMany,
)


@dataclass
class Tenant:
    """Rail class `Tenant` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    name: OgStr
    slug: OgStr
    max_users: Optional[OgInt]
    aktiv: Optional[OgBool]
    logo_path: Optional[OgStr]
    logo_mail_path: Optional[OgStr]
    created_at: Optional[OgDateTime]
    branche: Optional[OgStr]
    is_test: Optional[OgBool]
    is_anbieter: Optional[OgBool]
    mod_tresor: Optional[OgBool]
    mod_stundenzettel: Optional[OgBool]
    mod_multitimer: Optional[OgBool]
    mod_fahrtenbuch: Optional[OgBool]
    mod_wiedervorlage: Optional[OgBool]
    mod_notizbuch: Optional[OgBool]
    mod_wartung: Optional[OgBool]
    mod_abo: Optional[OgBool]
    mod_inventar: Optional[OgBool]
    mod_abnahme: Optional[OgBool]
    mod_gobd: Optional[OgBool]
    mod_referral: Optional[OgBool]
    mod_kaltakquise: Optional[OgBool]
    mod_woa_service: Optional[OgBool]
    mod_erp: Optional[OgBool]
    mod_dms: Optional[OgBool]
    mod_rustdesk: Optional[OgBool]
    mod_rustdesk_server: Optional[OgBool]
    erp_gobd_festschreibung: Optional[OgBool]
    erp_mod_stammdaten: Optional[OgBool]
    erp_mod_fibu: Optional[OgBool]
    erp_mod_bank: Optional[OgBool]
    erp_mod_steuer: Optional[OgBool]
    erp_mod_lager: Optional[OgBool]
    erp_mod_dms: Optional[OgBool]
    erp_mod_reporting: Optional[OgBool]
    erp_mod_pos: Optional[OgBool]
    erp_mod_zugferd: Optional[OgBool]
    erp_mod_lohn: Optional[OgBool]
    erp_mod_shop: Optional[OgBool]
    erp_mod_crm: Optional[OgBool]
    erp_mod_stammdaten_gesperrt: Optional[OgBool]
    erp_mod_fibu_gesperrt: Optional[OgBool]
    erp_mod_bank_gesperrt: Optional[OgBool]
    erp_mod_steuer_gesperrt: Optional[OgBool]
    erp_mod_lager_gesperrt: Optional[OgBool]
    erp_mod_dms_gesperrt: Optional[OgBool]
    erp_mod_reporting_gesperrt: Optional[OgBool]
    erp_mod_pos_gesperrt: Optional[OgBool]
    erp_mod_zugferd_gesperrt: Optional[OgBool]
    erp_mod_crm_gesperrt: Optional[OgBool]
    erp_mod_lohn_gesperrt: Optional[OgBool]
    erp_mod_shop_gesperrt: Optional[OgBool]

@dataclass
class User:
    """Rail class `User` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    username: OgStr
    password_hash: OgStr
    firstname: Optional[OgStr]
    lastname: Optional[OgStr]
    email: Optional[OgStr]
    phone: Optional[OgStr]
    ma_rabatt: Optional[OgFloat]
    is_admin: Optional[OgBool]
    is_superadmin: Optional[OgBool]
    perm_dashboard_umsaetze: OgBool
    perm_buchhaltung: OgBool
    perm_statistik: OgBool
    perm_einstellungen: OgBool
    perm_dms: OgBool
    perm_serverschutz: OgBool
    perm_erp_stammdaten: OgBool
    perm_erp_buchen: OgBool
    perm_erp_debitoren: OgBool
    perm_erp_kreditoren: OgBool
    perm_erp_bank: OgBool
    perm_erp_kasse: OgBool
    perm_erp_steuer: OgBool
    perm_erp_abschluss: OgBool
    perm_erp_anlagen: OgBool
    perm_erp_lager: OgBool
    perm_erp_einkauf: OgBool
    perm_erp_dms: OgBool
    perm_erp_compliance: OgBool
    perm_erp_lohn: OgBool
    failed_attempts: Optional[OgInt]
    locked_until: Optional[OgDateTime]
    scan_import_mode: Optional[OgStr]
    scan_import_username: Optional[OgStr]
    scan_import_pw_set_at: Optional[OgDateTime]
    scan_import_host: Optional[OgStr]
    scan_import_port: Optional[OgInt]
    vpn_enabled: OgBool
    vpn_ip: Optional[OgStr]
    vpn_pubkey: Optional[OgStr]
    vpn_created_at: Optional[OgDateTime]
    samba_enabled: OgBool
    samba_pw_set_at: Optional[OgDateTime]
    created_at: Optional[OgDateTime]
    tenant: ToOne["Tenant"]
    tresor_customer: ToOne["Customer"]

@dataclass
class Customer:
    """Rail class `Customer` — classid 0x02040003 (concept 0x0204, app 0x0003)."""
    CLASSID: ClassVar[int] = 0x02040003
    id: OgInt
    tenant_id: Optional[OgInt]
    kdnr: Optional[OgStr]
    quick_token: Optional[OgStr]
    quick_token_ts: Optional[OgStr]
    firma: Optional[OgStr]
    anrede: Optional[OgStr]
    vorname: Optional[OgStr]
    nachname: Optional[OgStr]
    mail_anrede: Optional[OgStr]
    strasse: Optional[OgStr]
    adresszusatz: Optional[OgStr]
    plz: Optional[OgStr]
    ort: Optional[OgStr]
    email: Optional[OgStr]
    telefon: Optional[OgStr]
    tresor_pw_hash: Optional[OgStr]
    tresor_pw_set_at: Optional[OgDateTime]
    tresor_pw_failed: Optional[OgInt]
    tresor_pw_locked_until: Optional[OgDateTime]
    zahlungsziel: Optional[OgInt]
    skonto_prozent: Optional[OgFloat]
    skonto_tage: Optional[OgInt]
    stundensatz: Optional[OgFloat]
    fahrt_km: Optional[OgFloat]
    fahrt_kosten: Optional[OgFloat]
    notizen: Optional[OgStr]
    aktiv: Optional[OgBool]
    kundentyp: Optional[OgStr]
    referral_code: Optional[OgStr]
    sepa_iban: Optional[OgStr]
    sepa_bic: Optional[OgStr]
    sepa_kontoinhaber: Optional[OgStr]
    sepa_mandat_ref: Optional[OgStr]
    sepa_mandat_datum: Optional[OgDate]
    sepa_mandat_typ: Optional[OgStr]
    sepa_mandat_status: Optional[OgStr]
    sepa_letzte_lastschrift: Optional[OgDate]
    sepa_pre_notification_tage: Optional[OgInt]
    preis_gruppe_id: Optional[OgInt]
    created_at: Optional[OgDateTime]
    workorders: ToMany["WorkOrder"]

@dataclass
class Project:
    """Rail class `Project` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    name: OgStr
    projekt_nr: Optional[OgStr]
    beschreibung: Optional[OgStr]
    status: Optional[OgStr]
    sort_order: Optional[OgInt]
    erstellt_am: Optional[OgDateTime]
    erstellt_von_id: Optional[OgInt]
    abgeschlossen_am: Optional[OgDateTime]
    customer: ToOne["Customer"]
    notes: ToOne["ProjectNote"]

@dataclass
class ProjectNote:
    """Rail class `ProjectNote` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    titel: Optional[OgStr]
    inhalt_text: Optional[OgScalar]
    inhalt_zeichnung: Optional[OgScalar]
    erstellt_am: Optional[OgDateTime]
    erstellt_von_id: Optional[OgInt]
    project: ToOne["Project"]

@dataclass
class ErpArticleCategory:
    """Rail class `ErpArticleCategory` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    name: OgStr
    icon: Optional[OgStr]
    sort_order: Optional[OgInt]
    created_at: Optional[OgDateTime]
    parent: ToMany["ErpArticleCategory"]

@dataclass
class ErpStorageLocation:
    """Rail class `ErpStorageLocation` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    name: OgStr
    kuerzel: Optional[OgStr]
    icon: Optional[OgStr]
    beschreibung: Optional[OgStr]
    sort_order: Optional[OgInt]
    aktiv: Optional[OgBool]
    created_at: Optional[OgDateTime]
    parent: ToMany["ErpStorageLocation"]

@dataclass
class Article:
    """Rail class `Article` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    artikelnr: Optional[OgStr]
    ean: Optional[OgStr]
    beschreibung: OgStr
    kategorie: Optional[OgStr]
    category_id: Optional[OgInt]
    storage_location_id: Optional[OgInt]
    einheit: Optional[OgStr]
    hersteller: Optional[OgStr]
    hersteller_anr: Optional[OgStr]
    bild_url: Optional[OgStr]
    bestand: Optional[OgFloat]
    mindestbestand: Optional[OgFloat]
    preis_netto: Optional[OgMoney]
    ek_preis: Optional[OgMoney]
    mwst_satz: Optional[OgFloat]
    tax_rate_id: Optional[OgInt]
    lieferant: Optional[OgStr]
    lieferant_anr: Optional[OgStr]
    notizen: Optional[OgStr]
    typ: Optional[OgStr]
    aktiv: Optional[OgBool]
    dauer_minuten: Optional[OgInt]
    preis_schema_id: Optional[OgInt]
    vk_preis_manuell: Optional[OgBool]
    listenpreis: Optional[OgMoney]
    uvp: Optional[OgMoney]
    gewicht_kg: Optional[OgMoney]
    herkunftsland: Optional[OgStr]
    zolltarifnr: Optional[OgStr]
    langbeschreibung: Optional[OgStr]
    matchcode: Optional[OgStr]
    warengruppe: Optional[OgStr]
    warengruppe_nr: Optional[OgStr]
    gefahrgut: Optional[OgBool]
    gefahrgut_un_nr: Optional[OgStr]
    gefahrgut_klasse: Optional[OgStr]
    auslaufartikel: Optional[OgBool]
    auslaufdatum: Optional[OgDate]
    deeplink: Optional[OgStr]
    shop_active: Optional[OgBool]
    shop_product_type: Optional[OgStr]
    shop_category_id: Optional[OgInt]
    shop_long_desc_html: Optional[OgStr]
    shop_saas_meta_json: Optional[OgScalar]
    shop_saas_package_id: Optional[OgInt]
    shop_digital_path: Optional[OgStr]
    shop_meta_title: Optional[OgStr]
    shop_meta_description: Optional[OgStr]
    shop_slug: Optional[OgStr]
    shop_payment_methods_csv: Optional[OgStr]
    shop_free_shipping: OgBool
    shop_extra_user_price_cents: Optional[OgInt]
    shop_addon_article_ids: Optional[OgScalar]
    shop_included_users: OgInt
    omd_tax_code: Optional[OgInt]

@dataclass
class WorkOrder:
    """Rail class `WorkOrder` — classid 0x02020003 (concept 0x0202, app 0x0003)."""
    CLASSID: ClassVar[int] = 0x02020003
    id: OgInt
    tenant_id: Optional[OgInt]
    customer_id: OgInt
    created_by: Optional[OgInt]
    doc_type: Optional[OgStr]
    status: Optional[OgStr]
    angebot_nr: Optional[OgStr]
    auftrags_nr: Optional[OgStr]
    workorder_nr: Optional[OgStr]
    rechnung_nr: Optional[OgStr]
    gutschrift_nr: Optional[OgStr]
    sammelrechnung_id: Optional[OgInt]
    datum: Optional[OgDate]
    zeit_start: Optional[OgStr]
    zeit_ende: Optional[OgStr]
    anfahrten: Optional[OgFloat]
    mitarbeiter: Optional[OgFloat]
    pause_h: Optional[OgFloat]
    zusatz_h: Optional[OgFloat]
    betreff: Optional[OgStr]
    notizen: Optional[OgStr]
    intern_notizen: Optional[OgStr]
    bezahlt: Optional[OgBool]
    bezahlt_am: Optional[OgDate]
    bezahlt_betrag: Optional[OgMoney]
    mahnstufe: Optional[OgInt]
    letzte_mahnung: Optional[OgDate]
    erfuellung_bis: Optional[OgDate]
    unterschrift: Optional[OgStr]
    signed_at: Optional[OgDateTime]
    signed_ip: Optional[OgStr]
    signed_user_agent: Optional[OgStr]
    zahlungsart: Optional[OgStr]
    anzahlung_prozent: Optional[OgFloat]
    anzahlung_betrag: Optional[OgFloat]
    anzahlung_bezahlt: Optional[OgBool]
    anzahlung_bezahlt_am: Optional[OgDate]
    kleinunternehmer_snapshot: Optional[OgBool]
    zahlungsziel_tage_snapshot: Optional[OgInt]
    gesamt_rabatt_prozent: Optional[OgFloat]
    gesamt_rabatt_betrag: Optional[OgFloat]
    skonto_prozent_snapshot: Optional[OgFloat]
    skonto_tage_snapshot: Optional[OgInt]
    skonto_ausweisen: Optional[OgBool]
    skonto_aufschlag: Optional[OgBool]
    skonto_aufschlag_faktor: Optional[OgFloat]
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]
    project: ToOne["Project"]
    positionen: ToMany["Position"]
    activities: ToMany["Activity"]
    pictures: ToMany["Picture"]
    history: ToMany["HistoryEntry"]
    acceptance_protocols: ToOne["AcceptanceProtocol"]
    child_workorders: ToMany["WorkOrder"]

@dataclass
class Position:
    """Rail class `Position` — classid 0x02010003 (concept 0x0201, app 0x0003)."""
    CLASSID: ClassVar[int] = 0x02010003
    id: OgInt
    workorder_id: OgInt
    article_id: Optional[OgInt]
    sort_order: Optional[OgInt]
    pos_typ: Optional[OgStr]
    beschreibung: Optional[OgStr]
    menge: Optional[OgFloat]
    einheit: Optional[OgStr]
    einzelpreis: Optional[OgMoney]
    mwst_satz: Optional[OgFloat]
    tax_rate_id: Optional[OgInt]
    versteckt: Optional[OgBool]
    is_optional: Optional[OgBool]
    customer_accepted_at: Optional[OgDateTime]
    rabatt_prozent: Optional[OgFloat]
    einzelpreis_vor_skonto: Optional[OgMoney]

@dataclass
class Activity:
    """Rail class `Activity` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    workorder_id: OgInt
    geraet: Optional[OgStr]
    beschreibung: Optional[OgStr]
    logbuch: Optional[OgBool]
    intern: Optional[OgBool]
    created_at: Optional[OgDateTime]
    acceptance_items: ToOne["AcceptanceItem"]

@dataclass
class Picture:
    """Rail class `Picture` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    workorder_id: OgInt
    dateiname: Optional[OgStr]
    beschreibung: Optional[OgStr]
    logbuch: Optional[OgBool]
    an_kunde: Optional[OgBool]
    created_at: Optional[OgDateTime]

@dataclass
class Document:
    """Rail class `Document` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    dateiname: OgStr
    original_name: OgStr
    beschreibung: Optional[OgStr]
    mime_type: Optional[OgStr]
    size_bytes: Optional[OgInt]
    an_kunde: Optional[OgBool]
    uploaded_by: Optional[OgInt]
    uploaded_at: Optional[OgDateTime]
    customer: ToOne["Customer"]
    workorder: ToOne["WorkOrder"]

@dataclass
class AcceptanceProtocol:
    """Rail class `AcceptanceProtocol` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    sequenz_nr: OgInt
    vorgaenger_id: Optional[OgInt]
    aktiv: Optional[OgBool]
    gesamt_abgenommen: Optional[OgBool]
    abnahme_datum: Optional[OgDate]
    abnahme_ort: Optional[OgStr]
    bemerkungen: Optional[OgStr]
    nachbesserungstermin: Optional[OgDate]
    unterschrift_kunde: Optional[OgStr]
    unterschrieben_am: Optional[OgDateTime]
    unterschrieben_von: Optional[OgStr]
    erstellt_am: Optional[OgDateTime]
    erstellt_von_id: Optional[OgInt]
    workorder: ToOne["WorkOrder"]
    items: ToOne["AcceptanceItem"]
    defects: ToOne["AcceptanceDefect"]

@dataclass
class AcceptanceItem:
    """Rail class `AcceptanceItem` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    abgenommen: Optional[OgBool]
    bemerkung: Optional[OgStr]
    bezeichnung: Optional[OgStr]
    status: Optional[OgStr]
    sort_order: Optional[OgInt]
    protocol: ToOne["AcceptanceProtocol"]
    activity: ToOne["Activity"]

@dataclass
class AcceptanceDefect:
    """Rail class `AcceptanceDefect` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    beschreibung: OgStr
    erfasst_am: Optional[OgDateTime]
    nachbesserung_bis: Optional[OgDate]
    behoben: Optional[OgBool]
    behoben_am: Optional[OgDateTime]
    intern_status: OgStr
    intern_status_am: Optional[OgDateTime]
    protocol: ToOne["AcceptanceProtocol"]

@dataclass
class AcceptanceTemplate:
    """Rail class `AcceptanceTemplate` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    name: OgStr
    is_default: Optional[OgBool]
    aktiv: Optional[OgBool]
    erstellt_am: Optional[OgDateTime]
    items: ToOne["AcceptanceTemplateItem"]

@dataclass
class AcceptanceTemplateItem:
    """Rail class `AcceptanceTemplateItem` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    bezeichnung: OgStr
    sort_order: Optional[OgInt]
    template: ToOne["AcceptanceTemplate"]

@dataclass
class HistoryEntry:
    """Rail class `HistoryEntry` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    workorder_id: OgInt
    aktion: Optional[OgStr]
    details: Optional[OgStr]
    created_at: Optional[OgDateTime]
    user: ToOne["User"]

@dataclass
class LogbookEntry:
    """Rail class `LogbookEntry` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    workorder_id: Optional[OgInt]
    datum: OgDate
    abfahrt: Optional[OgStr]
    ankunft: Optional[OgStr]
    rueckfahrt: Optional[OgStr]
    zurueck: Optional[OgStr]
    start_km: Optional[OgFloat]
    ende_km: Optional[OgFloat]
    route: Optional[OgStr]
    zweck: Optional[OgStr]
    fahrzeug: Optional[OgStr]
    privat_anteil: Optional[OgFloat]
    created_at: Optional[OgDateTime]
    user: ToOne["User"]
    customer: ToOne["Customer"]

@dataclass
class NumberSequence:
    """Rail class `NumberSequence` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    name: OgStr
    prefix: Optional[OgStr]
    current: Optional[OgInt]
    padding: Optional[OgInt]

@dataclass
class Setting:
    """Rail class `Setting` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    key: OgStr
    value: Optional[OgStr]
    label: Optional[OgStr]
    override_master: OgInt

@dataclass
class CustomerPortalUser:
    """Rail class `CustomerPortalUser` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    username: OgStr
    password_hash: OgStr
    aktiv: Optional[OgBool]
    must_change_pw: Optional[OgBool]
    last_login: Optional[OgDateTime]
    created_at: Optional[OgDateTime]
    failed_attempts: Optional[OgInt]
    locked_until: Optional[OgDateTime]
    customer: ToOne["Customer"]

@dataclass
class PasswordEntry:
    """Rail class `PasswordEntry` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    created_by: Optional[OgInt]
    gruppe: Optional[OgStr]
    titel: OgStr
    benutzername: Optional[OgStr]
    passwort_enc: Optional[OgStr]
    url: Optional[OgStr]
    notizen_enc: Optional[OgStr]
    icon: Optional[OgStr]
    aktiv: Optional[OgBool]
    keepass_uid: Optional[OgStr]
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]
    customer: ToOne["Customer"]
    creator: ToOne["User"]

@dataclass
class TimeSheet:
    """Rail class `TimeSheet` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    source: Optional[OgStr]
    datum: OgDate
    minuten: Optional[OgInt]
    erfasst_von: Optional[OgStr]
    startzeit: Optional[OgScalar]
    endzeit: Optional[OgScalar]
    beschreibung: Optional[OgStr]
    timer_start: Optional[OgDateTime]
    timer_paused_at: Optional[OgDateTime]
    abgerechnet: Optional[OgBool]
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]
    customer: ToOne["Customer"]
    user: ToOne["User"]

@dataclass
class TaxReserve:
    """Rail class `TaxReserve` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    jahr: OgInt
    monat: OgInt
    quartal: Optional[OgInt]
    typ: Optional[OgStr]
    erledigt: Optional[OgBool]

@dataclass
class TimesheetActivity:
    """Rail class `TimesheetActivity` — classid 0x01030003 (concept 0x0103, app 0x0003)."""
    CLASSID: ClassVar[int] = 0x01030003
    id: OgInt
    beschreibung: OgStr
    created_at: Optional[OgDateTime]
    timesheet: ToOne["TimeSheet"]

@dataclass
class Reminder:
    """Rail class `Reminder` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    titel: OgStr
    beschreibung: Optional[OgStr]
    faellig_am: OgDate
    prioritaet: Optional[OgStr]
    erledigt: Optional[OgBool]
    erledigt_am: Optional[OgDateTime]
    erstellt_am: Optional[OgDateTime]
    zeit_von: Optional[OgStr]
    zeit_bis: Optional[OgStr]
    termin_typ: Optional[OgStr]
    wiederkehrend: Optional[OgBool]
    intervall: Optional[OgStr]
    intervall_tage: Optional[OgStr]
    intervall_tag: Optional[OgInt]
    intervall_monat: Optional[OgInt]
    intervall_alle: Optional[OgInt]
    token_cancel: Optional[OgStr]
    cancelled_at: Optional[OgDateTime]
    cancelled_ip: Optional[OgStr]
    crm_lead_id: Optional[OgInt]
    crm_contact_id: Optional[OgInt]
    crm_company_id: Optional[OgInt]
    crm_task_id: Optional[OgInt]
    project: ToOne["Project"]
    user: ToOne["User"]
    customer: ToOne["Customer"]
    workorder: ToOne["WorkOrder"]

@dataclass
class MaintenanceContract:
    """Rail class `MaintenanceContract` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    titel: OgStr
    beschreibung: Optional[OgStr]
    intervall: Optional[OgStr]
    preis_netto: Optional[OgFloat]
    mwst_satz: Optional[OgFloat]
    beginn: OgDate
    ende: Optional[OgDate]
    letzte_wartung: Optional[OgDate]
    naechste_wartung: Optional[OgDate]
    auto_rechnung: Optional[OgBool]
    aktiv: Optional[OgBool]
    notizen: Optional[OgStr]
    created_at: Optional[OgDateTime]
    customer: ToMany["Customer"]

@dataclass
class RecurringInvoice:
    """Rail class `RecurringInvoice` — classid 0x02020003 (concept 0x0202, app 0x0003)."""
    CLASSID: ClassVar[int] = 0x02020003
    id: OgInt
    tenant_id: Optional[OgInt]
    titel: OgStr
    beschreibung: Optional[OgStr]
    intervall: Optional[OgStr]
    preis_netto: Optional[OgFloat]
    mwst_satz: Optional[OgFloat]
    naechste_ausfuehrung: OgDate
    letzte_ausfuehrung: Optional[OgDate]
    auto_versand: Optional[OgBool]
    aktiv: Optional[OgBool]
    notizen: Optional[OgStr]
    created_at: Optional[OgDateTime]
    customer: ToOne["Customer"]
    contract: ToOne["MaintenanceContract"]

@dataclass
class Device:
    """Rail class `Device` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    kategorie: Optional[OgStr]
    hersteller: Optional[OgStr]
    modell: Optional[OgStr]
    seriennummer: Optional[OgStr]
    hostname: Optional[OgStr]
    ip_adresse: Optional[OgStr]
    mac_adresse: Optional[OgStr]
    standort: Optional[OgStr]
    kaufdatum: Optional[OgDate]
    garantie_bis: Optional[OgDate]
    firmware: Optional[OgStr]
    zugangsdaten: Optional[OgStr]
    notizen: Optional[OgStr]
    letzte_wartung: Optional[OgDate]
    status: Optional[OgStr]
    aktiv: Optional[OgBool]
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]
    customer: ToMany["Customer"]

@dataclass
class KummerkastenEntry:
    """Rail class `KummerkastenEntry` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    source: Optional[OgStr]
    typ: Optional[OgStr]
    titel: OgStr
    beschreibung: Optional[OgStr]
    prioritaet: Optional[OgStr]
    status: Optional[OgStr]
    admin_kommentar: Optional[OgStr]
    erstellt_am: Optional[OgDateTime]
    aktualisiert_am: Optional[OgDateTime]
    user: ToOne["User"]
    tenant: ToOne["Tenant"]
    customer: ToOne["Customer"]

@dataclass
class PortalViewState:
    """Rail class `PortalViewState` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    customer_id: OgInt
    kind: OgStr
    last_seen_at: Optional[OgDateTime]

@dataclass
class ReferralLog:
    """Rail class `ReferralLog` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    referral_code: OgStr
    empfaenger_email: Optional[OgStr]
    empfaenger_name: Optional[OgStr]
    versendet_am: Optional[OgDateTime]
    versendet_von: Optional[OgInt]
    status: Optional[OgStr]
    converted_at: Optional[OgDateTime]
    converted_manually: Optional[OgBool]
    converted_note: Optional[OgStr]
    proposed_by: Optional[OgInt]
    proposed_at: Optional[OgDateTime]
    converted_tenant_id: Optional[OgInt]
    customer: ToOne["Customer"]
    tenant: ToOne["Tenant"]

@dataclass
class ColdLead:
    """Rail class `ColdLead` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    firma: OgStr
    ansprechpartner: Optional[OgStr]
    branche: Optional[OgStr]
    strasse: Optional[OgStr]
    plz: Optional[OgStr]
    ort: Optional[OgStr]
    land: Optional[OgStr]
    telefon: Optional[OgStr]
    mobil: Optional[OgStr]
    email: Optional[OgStr]
    webseite: Optional[OgStr]
    quelle: Optional[OgStr]
    briefanrede: Optional[OgStr]
    mitarbeiterzahl: Optional[OgStr]
    notiz_kurz: Optional[OgStr]
    newsletter_sperre: Optional[OgBool]
    newsletter_sperre_grund: Optional[OgStr]
    newsletter_sperre_am: Optional[OgDateTime]
    source_customer_id: Optional[OgInt]
    status: Optional[OgStr]
    wiedervorlage_am: Optional[OgDate]
    notizen: Optional[OgStr]
    converted_customer_id: Optional[OgInt]
    converted_at: Optional[OgDateTime]
    created_at: Optional[OgDateTime]
    created_by: Optional[OgInt]
    updated_at: Optional[OgDateTime]
    activities: ToMany["ColdLeadActivity"]

@dataclass
class ColdLeadActivity:
    """Rail class `ColdLeadActivity` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    lead_id: OgInt
    typ: Optional[OgStr]
    text: Optional[OgStr]
    mail_subject: Optional[OgStr]
    mail_to: Optional[OgStr]
    status_from: Optional[OgStr]
    status_to: Optional[OgStr]
    created_at: Optional[OgDateTime]
    created_by: Optional[OgInt]

@dataclass
class ColdCampaign:
    """Rail class `ColdCampaign` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    beschreibung: Optional[OgStr]
    status: Optional[OgStr]
    mail_subject: Optional[OgStr]
    mail_template_html: Optional[OgStr]
    created_at: Optional[OgDateTime]
    created_by: Optional[OgInt]
    updated_at: Optional[OgDateTime]
    leads: ToMany["ColdCampaignLead"]

@dataclass
class ColdCampaignLead:
    """Rail class `ColdCampaignLead` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    campaign_id: OgInt
    sent_at: Optional[OgDateTime]
    sent_status: Optional[OgStr]
    sent_error: Optional[OgStr]
    added_at: Optional[OgDateTime]
    added_by: Optional[OgInt]
    lead: ToOne["ColdLead"]

@dataclass
class ServicePackage:
    """Rail class `ServicePackage` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    beschreibung: Optional[OgStr]
    rechnungs_titel: Optional[OgStr]
    rechnungs_text: Optional[OgStr]
    preis_monthly: Optional[OgFloat]
    preis_quarterly: Optional[OgFloat]
    preis_half_yearly: Optional[OgFloat]
    preis_yearly: Optional[OgFloat]
    mwst_satz: Optional[OgFloat]
    free_months_default: Optional[OgInt]
    aktiv: Optional[OgBool]
    sort_order: Optional[OgInt]
    created_at: Optional[OgDateTime]
    with_mail_templates: Optional[OgBool]
    with_demo_data: Optional[OgBool]
    required_server_type: Optional[OgStr]

@dataclass
class RentedServer:
    """Rail class `RentedServer` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    provider: Optional[OgStr]
    hostname: Optional[OgStr]
    ip_address: Optional[OgStr]
    woa_url: Optional[OgStr]
    dns_eintrag: Optional[OgStr]
    ssh_user: Optional[OgStr]
    ssh_port: Optional[OgInt]
    miete_netto: Optional[OgFloat]
    miete_intervall: Optional[OgStr]
    ssh_password_enc: Optional[OgStr]
    root_password_enc: Optional[OgStr]
    notizen_enc: Optional[OgStr]
    master_api_url: Optional[OgStr]
    master_api_token: Optional[OgStr]
    verify_ssl: OgBool
    erp_ocr_token: Optional[OgStr]
    last_sync_at: Optional[OgDateTime]
    last_sync_status: Optional[OgStr]
    last_sync_message: Optional[OgStr]
    last_software_version: Optional[OgStr]
    last_health_at: Optional[OgDateTime]
    last_health_payload: Optional[OgStr]
    sa_username: Optional[OgStr]
    sa_password_enc: Optional[OgStr]
    installed_at: Optional[OgDateTime]
    tenant_slug_remote: Optional[OgStr]
    is_master: Optional[OgBool]
    aktiv: Optional[OgBool]
    created_at: Optional[OgDateTime]
    server_type: Optional[OgStr]
    max_tenants: Optional[OgInt]
    tenant_count: Optional[OgInt]
    tenant_count_at: Optional[OgDateTime]
    push_failure_count: OgInt
    push_last_failure_at: Optional[OgDateTime]
    push_last_error: Optional[OgStr]
    push_disabled: OgBool
    push_disabled_at: Optional[OgDateTime]

@dataclass
class ServiceContract:
    """Rail class `ServiceContract` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    intervall: Optional[OgStr]
    vertrag_start: OgDate
    rechnung_ab: OgDate
    naechste_rechnung: OgDate
    individualpreis_netto: Optional[OgFloat]
    rabatt_prozent: Optional[OgFloat]
    free_months_remaining: Optional[OgInt]
    credit_eur: Optional[OgFloat]
    bonus_pending_eur: Optional[OgFloat]
    sales_partner_id: Optional[OgInt]
    provision_pct: Optional[OgFloat]
    commission_until: Optional[OgDate]
    auto_versand: Optional[OgBool]
    aktiv: Optional[OgBool]
    gekuendigt_am: Optional[OgDate]
    geloescht_am: Optional[OgDateTime]
    notizen: Optional[OgStr]
    created_at: Optional[OgDateTime]
    last_invoice_at: Optional[OgDateTime]
    last_invoice_workorder_id: Optional[OgInt]
    shop_saas_config_id: Optional[OgInt]
    prorata_pending: OgBool
    customer: ToOne["Customer"]
    package: ToOne["ServicePackage"]
    server: ToOne["RentedServer"]

@dataclass
class SalesPartner:
    """Rail class `SalesPartner` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    tier: OgStr
    provision_pct: OgFloat
    aktiv: OgBool
    firma: Optional[OgStr]
    ansprechpartner: Optional[OgStr]
    email: Optional[OgStr]
    telefon: Optional[OgStr]
    is_service_techniker: OgBool
    strasse: Optional[OgStr]
    plz: Optional[OgStr]
    ort: Optional[OgStr]
    ustid: Optional[OgStr]
    steuer_status: Optional[OgStr]
    iban: Optional[OgStr]
    bic: Optional[OgStr]
    bank_name: Optional[OgStr]
    commission_months: Optional[OgInt]
    aktiv_ab: Optional[OgDate]
    aktiv_bis: Optional[OgDate]
    notizen: Optional[OgStr]
    vertrag_pdf_path: Optional[OgStr]
    vertrag_versendet_am: Optional[OgDateTime]
    vertrag_versand_method: Optional[OgStr]
    vertrag_signatur_token: Optional[OgStr]
    vertrag_signiertes_pdf_path: Optional[OgStr]
    vertrag_signiert_am: Optional[OgDateTime]
    vertrag_signatur_data: Optional[OgStr]
    vertrag_signatur_ip: Optional[OgStr]
    vertrag_signatur_user_agent: Optional[OgStr]
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]

@dataclass
class PartnerCommission:
    """Rail class `PartnerCommission` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    payout_id: Optional[OgInt]
    tenant_id: OgInt
    basis_netto: OgFloat
    provision_pct: OgFloat
    betrag_netto: OgFloat
    status: OgStr
    pending_at: Optional[OgDateTime]
    earned_at: Optional[OgDateTime]
    paid_at: Optional[OgDate]
    paid_workorder_id: Optional[OgInt]
    cancelled_at: Optional[OgDateTime]
    cancelled_reason: Optional[OgStr]
    notes: Optional[OgStr]
    created_at: Optional[OgDateTime]
    partner: ToOne["SalesPartner"]
    contract: ToOne["ServiceContract"]
    source_workorder: ToOne["WorkOrder"]

@dataclass
class PartnerPayout:
    """Rail class `PartnerPayout` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    payout_nr: Optional[OgStr]
    tenant_id: OgInt
    paid_at: OgDate
    summe_netto: OgFloat
    summe_ust: Optional[OgFloat]
    summe_brutto: Optional[OgFloat]
    ust_satz: Optional[OgFloat]
    steuer_status_snapshot: Optional[OgStr]
    note: Optional[OgStr]
    pdf_path: Optional[OgStr]
    pdf_generated_at: Optional[OgDateTime]
    mail_sent_at: Optional[OgDateTime]
    mail_sent_to: Optional[OgStr]
    created_by: Optional[OgInt]
    created_at: Optional[OgDateTime]
    partner: ToOne["SalesPartner"]
    commissions: ToMany["PartnerCommission"]

@dataclass
class ContractSetup:
    """Rail class `ContractSetup` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    setup_phase: OgStr
    partner_decision: Optional[OgStr]
    progress_pct: Optional[OgInt]
    subdomain: Optional[OgStr]
    server_ip: Optional[OgStr]
    dns_recipient: Optional[OgStr]
    dns_bcc: Optional[OgStr]
    dns_mail_sent_at: Optional[OgDateTime]
    dns_confirmed_at: Optional[OgDateTime]
    dns_confirmed_by: Optional[OgInt]
    server_ready_at: Optional[OgDateTime]
    server_ready_by: Optional[OgInt]
    server_notes: Optional[OgStr]
    ssh_host: Optional[OgStr]
    ssh_port: Optional[OgInt]
    ssh_user: Optional[OgStr]
    package_uploaded_at: Optional[OgDateTime]
    package_path_remote: Optional[OgStr]
    package_filename: Optional[OgStr]
    target_tenant_id: Optional[OgInt]
    target_tenant_name: Optional[OgStr]
    target_tenant_slug: Optional[OgStr]
    target_branche: Optional[OgStr]
    welcome_recipient: Optional[OgStr]
    welcome_sent_at: Optional[OgDateTime]
    admin_username: Optional[OgStr]
    onboarding_recipient: Optional[OgStr]
    onboarding_bcc: Optional[OgStr]
    onboarding_sent_at: Optional[OgDateTime]
    onboarding_csv_attached: Optional[OgBool]
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]
    contract: ToMany["ServiceContract"]

@dataclass
class ContractSetupHistory:
    """Rail class `ContractSetupHistory` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    contract_id: OgInt
    action: OgStr
    phase_from: Optional[OgStr]
    phase_to: Optional[OgStr]
    user_id: Optional[OgInt]
    notes: Optional[OgStr]
    created_at: Optional[OgDateTime]

@dataclass
class AppVersion:
    """Rail class `AppVersion` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    version: OgStr
    notes: Optional[OgStr]
    set_by: Optional[OgInt]
    set_at: Optional[OgDateTime]
    is_current: Optional[OgBool]
    change_pdf_path: Optional[OgStr]
    change_pdf_sha256: Optional[OgStr]
    change_pdf_size: Optional[OgInt]
    audit_detail_pdf_path: Optional[OgStr]
    audit_detail_pdf_sha256: Optional[OgStr]
    audit_detail_pdf_size: Optional[OgInt]
    audit_layperson_pdf_path: Optional[OgStr]
    audit_layperson_pdf_sha256: Optional[OgStr]
    audit_layperson_pdf_size: Optional[OgInt]
    audit_run_id: Optional[OgInt]

@dataclass
class IpBlacklist:
    """Rail class `IpBlacklist` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    ip: OgStr
    grund: Optional[OgStr]
    aktiv: Optional[OgBool]
    auto: Optional[OgBool]
    expires_at: Optional[OgDateTime]
    created_at: Optional[OgDateTime]
    created_by: Optional[OgInt]
    last_hit_at: Optional[OgDateTime]
    hit_count: Optional[OgInt]
    safe_marked_by: Optional[OgInt]
    safe_marked_at: Optional[OgDateTime]
    safe_reason: Optional[OgStr]
    origin_server: Optional[OgStr]
    creator: ToOne["User"]
    safe_marker: ToOne["User"]

@dataclass
class LoginAudit:
    """Rail class `LoginAudit` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    username: Optional[OgStr]
    ip: Optional[OgStr]
    user_agent: Optional[OgStr]
    success: Optional[OgBool]
    reason: Optional[OgStr]
    created_at: Optional[OgDateTime]

@dataclass
class ScopeAuditBlock:
    """Rail class `ScopeAuditBlock` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    created_at: OgDateTime
    actor_user_id: Optional[OgInt]
    actor_username: Optional[OgStr]
    actor_tenant_id: Optional[OgInt]
    actor_is_admin: Optional[OgBool]
    actor_is_superadmin: Optional[OgBool]
    target_model: Optional[OgStr]
    target_id: Optional[OgInt]
    target_tenant_id: Optional[OgInt]
    route: Optional[OgStr]
    method: Optional[OgStr]
    reason: Optional[OgStr]
    ip: Optional[OgStr]
    user_agent: Optional[OgStr]

@dataclass
class SecurityAudit:
    """Rail class `SecurityAudit` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    app_version: Optional[OgStr]
    status: Optional[OgStr]
    report_mode: Optional[OgStr]
    audited_by: Optional[OgInt]
    audited_by_name: Optional[OgStr]
    audited_at: Optional[OgDateTime]
    approved_by: Optional[OgInt]
    approved_by_name: Optional[OgStr]
    approved_at: Optional[OgDateTime]
    results_json: Optional[OgStr]
    overall_status: Optional[OgStr]
    pass_count: Optional[OgInt]
    fail_count: Optional[OgInt]
    warn_count: Optional[OgInt]
    skip_count: Optional[OgInt]
    auditor_notes: Optional[OgStr]
    version_history_json: Optional[OgStr]
    signature_hash: Optional[OgStr]
    signed_at: Optional[OgDateTime]
    pdf_path: Optional[OgStr]
    pdf_sha256: Optional[OgStr]
    pdf_size: Optional[OgInt]
    sa_notified_at: Optional[OgDateTime]
    created_at: Optional[OgDateTime]
    auditor: ToOne["User"]
    approver: ToOne["User"]

@dataclass
class SyncHistory:
    """Rail class `SyncHistory` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    rented_server_id: OgInt
    started_at: OgDateTime
    finished_at: Optional[OgDateTime]
    action: Optional[OgStr]
    status: Optional[OgStr]
    files_changed: Optional[OgInt]
    backup_path: Optional[OgStr]
    error_message: Optional[OgStr]
    log_excerpt: Optional[OgStr]
    triggered_by: Optional[OgInt]
    server: ToOne["RentedServer"]

@dataclass
class UpdateJob:
    """Rail class `UpdateJob` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    status: Optional[OgStr]
    started_at: OgDateTime
    finished_at: Optional[OgDateTime]
    last_heartbeat: OgDateTime
    current_step: Optional[OgStr]
    progress_percent: Optional[OgInt]
    log_excerpt: Optional[OgStr]
    error_message: Optional[OgStr]
    triggered_by: Optional[OgInt]
    cancel_requested: OgBool
    server: ToOne["RentedServer"]

@dataclass
class UpdateSnapshot:
    """Rail class `UpdateSnapshot` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    snapshot_file: OgStr
    snapshot_size: Optional[OgInt]
    version_before: Optional[OgStr]
    version_after: Optional[OgStr]
    status: Optional[OgStr]
    log_excerpt: Optional[OgStr]
    created_at: OgDateTime
    restored_at: Optional[OgDateTime]
    rollback_note: Optional[OgStr]

@dataclass
class TerminVorschlag:
    """Rail class `TerminVorschlag` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    project_id: Optional[OgInt]
    defect_id: Optional[OgInt]
    titel: OgStr
    beschreibung: Optional[OgStr]
    termin_typ: Optional[OgStr]
    vorschlaege_json: Optional[OgStr]
    status: Optional[OgStr]
    accepted_index: Optional[OgInt]
    accepted_reminder_id: Optional[OgInt]
    accepted_at: Optional[OgDateTime]
    accepted_ip: Optional[OgStr]
    token_slot1: Optional[OgStr]
    token_slot2: Optional[OgStr]
    token_slot3: Optional[OgStr]
    token_decline: OgStr
    token_cancel: Optional[OgStr]
    cancelled_at: Optional[OgDateTime]
    cancelled_ip: Optional[OgStr]
    expires_at: OgDateTime
    erstellt_am: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]
    customer: ToOne["Customer"]
    user: ToOne["User"]
    workorder: ToOne["WorkOrder"]
    reminder: ToOne["Reminder"]

@dataclass
class TerminVorschlagBlocker:
    """Rail class `TerminVorschlagBlocker` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    slot_index: OgInt
    datum: OgDate
    zeit_von: Optional[OgStr]
    zeit_bis: Optional[OgStr]
    caldav_uid: OgStr
    pushed_at: Optional[OgDateTime]
    deleted_at: Optional[OgDateTime]
    erstellt_am: Optional[OgDateTime]
    tv: ToOne["TerminVorschlag"]

@dataclass
class HandbookFeature:
    """Rail class `HandbookFeature` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    titel: OgStr
    beschreibung: Optional[OgStr]
    kategorie: Optional[OgStr]
    version: Optional[OgStr]
    reihenfolge: Optional[OgInt]
    aktiv: Optional[OgBool]
    datum: Optional[OgDate]
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]

@dataclass
class BrancheTemplate:
    """Rail class `BrancheTemplate` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    branche: OgStr
    label: Optional[OgStr]
    html_content: Optional[OgStr]
    aktiv: Optional[OgBool]
    erstellt_am: Optional[OgDateTime]
    aktualisiert_am: Optional[OgDateTime]

@dataclass
class ServiceContractItem:
    """Rail class `ServiceContractItem` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    sort_order: Optional[OgInt]
    pos_typ: Optional[OgStr]
    titel: Optional[OgStr]
    beschreibung: Optional[OgStr]
    intervall: Optional[OgStr]
    preis_netto: Optional[OgFloat]
    menge: Optional[OgFloat]
    mwst_satz: Optional[OgFloat]
    rabatt_typ: Optional[OgStr]
    rabatt_wert: Optional[OgFloat]
    rabatt_bis: Optional[OgDate]
    rabatt_grund: Optional[OgStr]
    aktiv_ab: Optional[OgDate]
    aktiv_bis: Optional[OgDate]
    abgerechnet_bis: Optional[OgDate]
    sofort_rechnung: Optional[OgBool]
    aktiv: Optional[OgBool]
    created_at: Optional[OgDateTime]
    contract: ToOne["ServiceContract"]
    package: ToOne["ServicePackage"]

@dataclass
class ContractBonus:
    """Rail class `ContractBonus` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    bonus_typ: OgStr
    wert: Optional[OgFloat]
    grund: Optional[OgStr]
    aktiv_ab: Optional[OgDate]
    aktiv_bis: Optional[OgDate]
    verbraucht: Optional[OgBool]
    verbraucht_am: Optional[OgDateTime]
    verbraucht_in_workorder_id: Optional[OgInt]
    promo_code_id: Optional[OgInt]
    aktiv: Optional[OgBool]
    created_at: Optional[OgDateTime]
    created_by: Optional[OgInt]
    contract: ToMany["ServiceContract"]

@dataclass
class PromoCode:
    """Rail class `PromoCode` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    code: OgStr
    titel: Optional[OgStr]
    beschreibung: Optional[OgStr]
    code_typ: Optional[OgStr]
    wert: Optional[OgFloat]
    gueltig_ab: Optional[OgDate]
    gueltig_bis: Optional[OgDate]
    max_einloesungen: Optional[OgInt]
    aktuelle_einloesungen: Optional[OgInt]
    nur_neukunden: Optional[OgBool]
    min_vertragswert_netto: Optional[OgFloat]
    bonus_aktiv_monate: Optional[OgInt]
    aktiv: Optional[OgBool]
    created_at: Optional[OgDateTime]
    created_by: Optional[OgInt]

@dataclass
class OnboardingTemplate:
    """Rail class `OnboardingTemplate` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    kind: OgStr
    label: Optional[OgStr]
    subject: Optional[OgStr]
    html_content: Optional[OgStr]
    plain_body: Optional[OgStr]
    aktiv: Optional[OgBool]
    erstellt_am: Optional[OgDateTime]
    aktualisiert_am: Optional[OgDateTime]

@dataclass
class PartnerContractTemplate:
    """Rail class `PartnerContractTemplate` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    label: Optional[OgStr]
    html_content: Optional[OgStr]
    aktiv: Optional[OgBool]
    erstellt_am: Optional[OgDateTime]
    aktualisiert_am: Optional[OgDateTime]

@dataclass
class PortalAutoLoginToken:
    """Rail class `PortalAutoLoginToken` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    customer_portal_user_id: OgInt
    workorder_id: Optional[OgInt]
    token: OgStr
    created_at: OgDateTime
    expires_at: OgDateTime
    last_used_at: Optional[OgDateTime]
    revoked_at: Optional[OgDateTime]
    use_count: OgInt
    created_by: Optional[OgStr]
    scope: Optional[OgStr]
    permanent: Optional[OgBool]
    portal_user: ToOne["CustomerPortalUser"]

@dataclass
class MailTemplate:
    """Rail class `MailTemplate` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    kind: OgStr
    subject: OgStr
    html_body: OgStr
    plain_body: Optional[OgStr]
    updated_at: OgDateTime
    updated_by_user_id: Optional[OgInt]

@dataclass
class TresorPwResetToken:
    """Rail class `TresorPwResetToken` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    customer_id: OgInt
    token: OgStr
    created_at: OgDateTime
    expires_at: OgDateTime
    used_at: Optional[OgDateTime]
    created_by: Optional[OgStr]

@dataclass
class LegacyRouteKey:
    """Rail class `LegacyRouteKey` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    key_data: OgStr
    retired_at: OgDateTime
    expires_at: OgDateTime
    note: Optional[OgStr]

@dataclass
class RevokedToken:
    """Rail class `RevokedToken` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    token_hash: OgStr
    revoked_at: OgDateTime
    revoked_by_uid: Optional[OgInt]
    reason: Optional[OgStr]

@dataclass
class GeoblockSetting:
    """Rail class `GeoblockSetting` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    country_code: OgStr
    country_name: OgStr
    continent: Optional[OgStr]
    blocked: OgBool
    updated_at: Optional[OgDateTime]
    updated_by: Optional[OgInt]

@dataclass
class GeoblockAllowIP:
    """Rail class `GeoblockAllowIP` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    cidr: OgStr
    note: Optional[OgStr]
    created_at: Optional[OgDateTime]
    created_by: Optional[OgInt]

@dataclass
class GeoblockStat:
    """Rail class `GeoblockStat` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    country_code: OgStr
    stat_date: OgDate
    block_count: OgInt

@dataclass
class ClaudeLesson:
    """Rail class `ClaudeLesson` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    session_date: OgDate
    category: OgStr
    severity: OgStr
    title: OgStr
    body: OgStr
    tags: Optional[OgStr]
    related_patches: Optional[OgStr]
    is_active: Optional[OgBool]
    created_at: Optional[OgDateTime]
    created_by: Optional[OgStr]

@dataclass
class ClaudeStaticSection:
    """Rail class `ClaudeStaticSection` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    position: OgInt
    section_key: OgStr
    title: OgStr
    content: OgStr
    has_timestamp_placeholder: Optional[OgBool]
    is_active: Optional[OgBool]
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]

@dataclass
class TaxRate:
    """Rail class `TaxRate` — classid 0x02030003 (concept 0x0203, app 0x0003)."""
    CLASSID: ClassVar[int] = 0x02030003
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    prozent: OgFloat
    aktiv: OgBool
    is_default: OgBool
    sort_order: OgInt
    created_at: Optional[OgDateTime]

@dataclass
class ShiftTicket:
    """Rail class `ShiftTicket` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    token: OgStr
    valid_until: OgDateTime
    created_at: Optional[OgDateTime]
    created_by_id: Optional[OgInt]
    used_at: Optional[OgDateTime]
    customer: ToOne["Customer"]
    project: ToOne["Project"]
    assigned_user: ToOne["User"]
    reminder: ToOne["Reminder"]
    used_workorder: ToOne["WorkOrder"]

@dataclass
class ErpLegalProfile:
    """Rail class `ErpLegalProfile` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    rechtsform: Optional[OgStr]
    festschreib_periode: Optional[OgStr]
    kleinunternehmer: Optional[OgBool]
    besteuerungsart: Optional[OgStr]
    buchen_belegpflicht: Optional[OgBool]
    lager_fibu_methode: Optional[OgStr]
    lager_wareneinsatz_konto: Optional[OgStr]
    bank_sachkonto_auto_pi: Optional[OgBool]
    customer_bank_autosync: Optional[OgStr]
    handelsregister: Optional[OgStr]
    steuernummer: Optional[OgStr]
    ust_id: Optional[OgStr]
    default_currency: Optional[OgStr]
    currency_custom_code: Optional[OgStr]
    currency_custom_symbol: Optional[OgStr]
    datev_beraternr: Optional[OgStr]
    datev_mandantnr: Optional[OgStr]
    datev_wj_beginn: Optional[OgInt]
    erp_startdatum: Optional[OgDate]
    legacy_source: Optional[OgStr]
    legacy_object_id: Optional[OgStr]
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]
    imap_server: Optional[OgStr]
    imap_port: Optional[OgInt]
    imap_user: Optional[OgStr]
    imap_password: Optional[OgStr]
    imap_folder: Optional[OgStr]
    imap_ssl: Optional[OgBool]
    imap_aktiv: Optional[OgBool]
    imap_forward_to: Optional[OgStr]
    imap_interval: Optional[OgInt]
    gewerbesteuer_hebesatz: Optional[OgInt]
    erechnung_format: Optional[OgStr]

@dataclass
class ErpSupplierIban:
    """Rail class `ErpSupplierIban` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    supplier_id: OgInt
    iban: OgStr
    bic: Optional[OgStr]
    notiz: Optional[OgStr]
    created_at: Optional[OgDateTime]

@dataclass
class ErpFintsInstitute:
    """Rail class `ErpFintsInstitute` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    blz: OgStr
    bic: Optional[OgStr]
    institut: Optional[OgStr]
    ort: Optional[OgStr]
    rz: Optional[OgStr]
    organisation: Optional[OgStr]
    hbci_dns: Optional[OgStr]
    hbci_version: Optional[OgStr]
    pintan_url: Optional[OgStr]
    fints_version: Optional[OgStr]
    updated_at_src: Optional[OgDate]
    imported_at: Optional[OgDateTime]

@dataclass
class ErpExchangeRate:
    """Rail class `ErpExchangeRate` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    from_currency: Optional[OgStr]
    to_currency: Optional[OgStr]
    rate: Optional[OgMoney]
    valid_from: Optional[OgDate]
    valid_until: Optional[OgDate]
    source: Optional[OgStr]
    created_at: Optional[OgDateTime]

@dataclass
class ErpEstTarifParams:
    """Rail class `ErpEstTarifParams` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    jahr: OgInt
    grundfreibetrag: Optional[OgInt]
    zone2_bis: Optional[OgInt]
    zone3_bis: Optional[OgInt]
    zone4_bis: Optional[OgInt]
    z2a: Optional[OgStr]
    z2b: Optional[OgStr]
    z3a: Optional[OgStr]
    z3b: Optional[OgStr]
    z3c: Optional[OgStr]
    z4_abzug: Optional[OgStr]
    z5_abzug: Optional[OgStr]
    soli_freigrenze: Optional[OgInt]
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]

@dataclass
class ErpKuGrenzen:
    """Rail class `ErpKuGrenzen` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    jahr: OgInt
    grenze_vorjahr: Optional[OgInt]
    grenze_laufend: Optional[OgInt]
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]

@dataclass
class ErpLedgerLock:
    """Rail class `ErpLedgerLock` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    buchungsperiode: OgStr
    status: Optional[OgStr]
    locked_at: Optional[OgDateTime]
    locked_by_user_id: Optional[OgInt]
    hash_snapshot: Optional[OgStr]
    created_at: Optional[OgDateTime]

@dataclass
class ErpAuditTrail:
    """Rail class `ErpAuditTrail` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    seq_no: OgInt
    entity: OgStr
    entity_id: Optional[OgInt]
    aktion: OgStr
    user_id: Optional[OgInt]
    ip: Optional[OgStr]
    user_agent: Optional[OgStr]
    freitext: Optional[OgStr]
    before_hash: Optional[OgStr]
    after_hash: Optional[OgStr]
    created_at: Optional[OgDateTime]

@dataclass
class ErpChartOfAccounts:
    """Rail class `ErpChartOfAccounts` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    rahmen: OgStr
    aktiv: Optional[OgBool]
    gesperrt_ab: Optional[OgDateTime]
    created_at: Optional[OgDateTime]

@dataclass
class ErpAccount:
    """Rail class `ErpAccount` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    rahmen: OgStr
    kontonummer: OgStr
    bezeichnung: OgStr
    kontoart: Optional[OgStr]
    kontenklasse: Optional[OgInt]
    steuer_relevant: Optional[OgBool]
    ust_kennzeichen: Optional[OgStr]
    automatik_konto: Optional[OgBool]
    gesperrt: Optional[OgBool]
    sort_order: Optional[OgInt]
    currency: Optional[OgStr]
    legacy_source: Optional[OgStr]
    legacy_object_id: Optional[OgStr]
    created_at: Optional[OgDateTime]

@dataclass
class ErpCostCenter:
    """Rail class `ErpCostCenter` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    nummer: OgStr
    bezeichnung: Optional[OgStr]
    aktiv: Optional[OgBool]
    parent_id: Optional[OgInt]
    created_at: Optional[OgDateTime]

@dataclass
class ErpFiscalYear:
    """Rail class `ErpFiscalYear` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    jahr: OgInt
    beginn: OgDate
    ende: OgDate
    status: Optional[OgStr]
    created_at: Optional[OgDateTime]

@dataclass
class ErpPeriod:
    """Rail class `ErpPeriod` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    fiscal_year_id: Optional[OgInt]
    periode_key: OgStr
    bezeichnung: Optional[OgStr]
    status: Optional[OgStr]
    created_at: Optional[OgDateTime]

@dataclass
class ErpTaxAccountMap:
    """Rail class `ErpTaxAccountMap` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    rahmen: OgStr
    ust_kennzeichen: OgStr
    beschreibung: Optional[OgStr]
    ust_konto: Optional[OgStr]
    vst_konto: Optional[OgStr]
    steuer_konto: Optional[OgStr]
    prozent: Optional[OgMoney]
    gueltig_ab: Optional[OgDate]
    gueltig_bis: Optional[OgDate]
    created_at: Optional[OgDateTime]

@dataclass
class ErpJournalEntry:
    """Rail class `ErpJournalEntry` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    belegnummer: OgStr
    belegdatum: OgDate
    buchungsdatum: OgDate
    buchungstext: Optional[OgStr]
    erfasst_von_user_id: Optional[OgInt]
    festgeschrieben: Optional[OgBool]
    festgeschrieben_am: Optional[OgDateTime]
    storno_of_id: Optional[OgInt]
    herkunft: Optional[OgStr]
    herkunft_ref_id: Optional[OgInt]
    currency: Optional[OgStr]
    created_at: Optional[OgDateTime]
    lines: ToMany["ErpJournalLine"]

@dataclass
class ErpJournalLine:
    """Rail class `ErpJournalLine` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    entry_id: OgInt
    konto: OgStr
    gegenkonto: Optional[OgStr]
    soll_betrag: Optional[OgMoney]
    haben_betrag: Optional[OgMoney]
    steuer_betrag: Optional[OgMoney]
    ust_kennzeichen: Optional[OgStr]
    kostenstelle_id: Optional[OgInt]
    zeilentext: Optional[OgStr]

@dataclass
class ErpDebitorAccount:
    """Rail class `ErpDebitorAccount` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    customer_id: OgInt
    kontonummer: OgStr
    created_at: Optional[OgDateTime]

@dataclass
class ErpSupplier:
    """Rail class `ErpSupplier` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    name: OgStr
    anschrift: Optional[OgStr]
    ust_id: Optional[OgStr]
    steuernummer: Optional[OgStr]
    iban: Optional[OgStr]
    bic: Optional[OgStr]
    zahlungsziel: Optional[OgInt]
    skonto_tage: Optional[OgInt]
    skonto_prozent: Optional[OgFloat]
    kreditorenkonto: Optional[OgStr]
    aufwandskonto_default: Optional[OgStr]
    ansprechpartner: Optional[OgStr]
    ap_position: Optional[OgStr]
    telefon: Optional[OgStr]
    email: Optional[OgStr]
    website: Optional[OgStr]
    strasse: Optional[OgStr]
    plz: Optional[OgStr]
    ort: Optional[OgStr]
    land: Optional[OgStr]
    unsere_kundennr: Optional[OgStr]
    notizen: Optional[OgStr]
    lieferzeit_tage: Optional[OgInt]
    mindestbestellwert: Optional[OgMoney]
    aktiv: Optional[OgBool]
    created_at: Optional[OgDateTime]

@dataclass
class ErpPurchaseInvoice:
    """Rail class `ErpPurchaseInvoice` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    rechnungsnr: OgStr
    belegdatum: OgDate
    eingangsdatum: Optional[OgDate]
    faellig_am: Optional[OgDate]
    netto: Optional[OgMoney]
    steuer: Optional[OgMoney]
    brutto: Optional[OgMoney]
    currency: Optional[OgStr]
    aufwandskonto: Optional[OgStr]
    buchungstext: Optional[OgStr]
    split_buchung: Optional[OgStr]
    geprueft: Optional[OgBool]
    geprueft_von: Optional[OgInt]
    geprueft_am: Optional[OgDateTime]
    dokument_pfad: Optional[OgStr]
    status: Optional[OgStr]
    bezahlt: Optional[OgBool]
    bezahlt_am: Optional[OgDate]
    journal_entry_id: Optional[OgInt]
    created_at: Optional[OgDateTime]
    skontofrist: Optional[OgInt]
    skontosatz: Optional[OgMoney]
    supplier: ToOne["ErpSupplier"]

@dataclass
class ErpSupplierArticle:
    """Rail class `ErpSupplierArticle` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    lieferanten_artikelnr: Optional[OgStr]
    ek_preis: Optional[OgMoney]
    currency: Optional[OgStr]
    mindestmenge: Optional[OgInt]
    staffelpreise: Optional[OgStr]
    lieferzeit_tage: Optional[OgInt]
    ist_hauptlieferant: Optional[OgBool]
    aktiv: Optional[OgBool]
    created_at: Optional[OgDateTime]
    supplier: ToOne["ErpSupplier"]
    article: ToOne["Article"]

@dataclass
class ErpPurchaseOrder:
    """Rail class `ErpPurchaseOrder` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    bestellnummer: OgStr
    status: Optional[OgStr]
    bestelldatum: Optional[OgDate]
    liefertermin: Optional[OgDate]
    netto_summe: Optional[OgMoney]
    steuer_summe: Optional[OgMoney]
    brutto_summe: Optional[OgMoney]
    currency: Optional[OgStr]
    notizen: Optional[OgStr]
    purchase_invoice_id: Optional[OgInt]
    erstellt_von: Optional[OgInt]
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]
    supplier: ToOne["ErpSupplier"]
    lines: ToMany["ErpPurchaseOrderLine"]

@dataclass
class ErpPurchaseOrderLine:
    """Rail class `ErpPurchaseOrderLine` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    purchase_order_id: OgInt
    artikelnr: Optional[OgStr]
    bezeichnung: Optional[OgStr]
    menge: OgFloat
    ek_preis: Optional[OgMoney]
    steuersatz: Optional[OgMoney]
    currency: Optional[OgStr]
    geliefert_menge: Optional[OgFloat]
    notiz: Optional[OgStr]
    article: ToOne["Article"]

@dataclass
class ErpMaterialBedarf:
    """Rail class `ErpMaterialBedarf` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    menge_position: Optional[OgFloat]
    menge_bestand: Optional[OgFloat]
    menge_bedarf: Optional[OgFloat]
    menge_bestellt: Optional[OgFloat]
    menge_geliefert: Optional[OgFloat]
    einheit: Optional[OgStr]
    best_ek: Optional[OgMoney]
    status: Optional[OgStr]
    workorder_nr: Optional[OgStr]
    artikel_bez: Optional[OgStr]
    created_at: Optional[OgDateTime]
    created_by: Optional[OgInt]
    notiz: Optional[OgStr]
    workorder: ToOne["WorkOrder"]
    article: ToOne["Article"]
    best_supplier: ToOne["ErpSupplier"]
    position: ToOne["Position"]
    purchase_order: ToOne["ErpPurchaseOrder"]

@dataclass
class ErpCashRegister:
    """Rail class `ErpCashRegister` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    name: Optional[OgStr]
    sachkonto: Optional[OgStr]
    currency: Optional[OgStr]
    aktiv: Optional[OgBool]
    anfangssaldo: Optional[OgMoney]
    created_at: Optional[OgDateTime]

@dataclass
class ErpCashEntry:
    """Rail class `ErpCashEntry` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    lfd_nr: OgInt
    datum: OgDate
    vorgang: OgStr
    betrag: OgMoney
    gegenkonto: Optional[OgStr]
    zweck: Optional[OgStr]
    erfasst_von_user_id: Optional[OgInt]
    festgeschrieben: Optional[OgBool]
    storno_of_id: Optional[OgInt]
    journal_entry_id: Optional[OgInt]
    created_at: Optional[OgDateTime]
    register: ToOne["ErpCashRegister"]

@dataclass
class ErpDocument:
    """Rail class `ErpDocument` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    original_name: OgStr
    stored_name: OgStr
    mime_type: Optional[OgStr]
    size_bytes: Optional[OgInt]
    integrity_hash: Optional[OgStr]
    belegtyp: Optional[OgStr]
    beleg_status: Optional[OgStr]
    steuerberater_relevant: Optional[OgBool]
    beschreibung: Optional[OgStr]
    customer_id: Optional[OgInt]
    purchase_invoice_id: Optional[OgInt]
    asset_id: Optional[OgInt]
    journal_entry_id: Optional[OgInt]
    legacy_source: Optional[OgStr]
    legacy_object_id: Optional[OgStr]
    legacy_tree_id: Optional[OgStr]
    legacy_filename: Optional[OgStr]
    legacy_imported_at: Optional[OgDateTime]
    uploaded_by: Optional[OgInt]
    uploaded_at: Optional[OgDateTime]
    kategorie_id: Optional[OgInt]
    periode: Optional[OgStr]
    tags: Optional[OgStr]
    beleg_datum: Optional[OgDate]
    lieferant_id: Optional[OgInt]
    workorder_id: Optional[OgInt]
    bank_tx_id: Optional[OgInt]
    ocr_status: Optional[OgStr]

@dataclass
class ErpDocumentFulltext:
    """Rail class `ErpDocumentFulltext` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    quelle: Optional[OgStr]
    text: Optional[OgStr]
    indexed_at: Optional[OgDateTime]
    document: ToOne["ErpDocument"]

@dataclass
class ErpDmsKategorie:
    """Rail class `ErpDmsKategorie` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    parent_id: Optional[OgInt]
    name: OgStr
    beschreibung: Optional[OgStr]
    icon: Optional[OgStr]
    sort_order: Optional[OgInt]
    aktiv: Optional[OgBool]
    created_at: Optional[OgDateTime]
    children: ToMany["ErpDmsKategorie"]

@dataclass
class ErpDmsAudit:
    """Rail class `ErpDmsAudit` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    document_id: OgInt
    user_id: Optional[OgInt]
    aktion: OgStr
    von_status: Optional[OgStr]
    zu_status: Optional[OgStr]
    kommentar: Optional[OgStr]
    created_at: Optional[OgDateTime]

@dataclass
class SambaShare:
    """Rail class `SambaShare` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    name: OgStr
    path: OgStr
    comment: Optional[OgStr]
    browseable: Optional[OgBool]
    aktiv: Optional[OgBool]
    created_at: Optional[OgDateTime]

@dataclass
class SambaShareAcl:
    """Rail class `SambaShareAcl` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    can_read: Optional[OgBool]
    can_write: Optional[OgBool]
    share: ToOne["SambaShare"]
    user: ToOne["User"]

@dataclass
class ScanRequest:
    """Rail class `ScanRequest` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    user_id: OgInt
    ziel_typ: OgStr
    ziel_id: Optional[OgInt]
    status: Optional[OgStr]
    document_id: Optional[OgInt]
    created_at: Optional[OgDateTime]
    completed_at: Optional[OgDateTime]
    expires_at: Optional[OgDateTime]
    hinweis: Optional[OgStr]

@dataclass
class EinsatzSession:
    """Rail class `EinsatzSession` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    pairing_code: OgStr
    qr_token: Optional[OgStr]
    code_expires_at: OgDateTime
    session_token: Optional[OgStr]
    user_id: Optional[OgInt]
    customer_id: Optional[OgInt]
    reminder_id: Optional[OgInt]
    workorder_id: Optional[OgInt]
    scope: Optional[OgStr]
    status: Optional[OgStr]
    paired_at: Optional[OgDateTime]
    expires_at: Optional[OgDateTime]
    last_activity: Optional[OgDateTime]
    ended_at: Optional[OgDateTime]
    ended_by: Optional[OgStr]
    browser_fp: Optional[OgStr]
    ip_address: Optional[OgStr]
    user_agent: Optional[OgStr]
    created_at: Optional[OgDateTime]

@dataclass
class ErpSerialCharge:
    """Rail class `ErpSerialCharge` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    typ: Optional[OgStr]
    nummer: OgStr
    mhd: Optional[OgDate]
    status: Optional[OgStr]
    workorder_id: Optional[OgInt]
    movement_id: Optional[OgInt]
    created_at: Optional[OgDateTime]
    article: ToOne["Article"]

@dataclass
class ErpEinvoiceImport:
    """Rail class `ErpEinvoiceImport` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    status: Optional[OgStr]
    filename: Optional[OgStr]
    dateityp: Optional[OgStr]
    raw_xml: Optional[OgStr]
    profil: Optional[OgStr]
    format_typ: Optional[OgStr]
    lieferant_name: Optional[OgStr]
    lieferant_ustid: Optional[OgStr]
    lieferant_iban: Optional[OgStr]
    rechnungsnr: Optional[OgStr]
    rechnungsdatum: Optional[OgDate]
    faelligkeit: Optional[OgDate]
    waehrung: Optional[OgStr]
    netto_betrag: Optional[OgMoney]
    steuer_betrag: Optional[OgMoney]
    brutto_betrag: Optional[OgMoney]
    steuer_prozent: Optional[OgMoney]
    verwendungszweck: Optional[OgStr]
    positionen_json: Optional[OgStr]
    purchase_invoice_id: Optional[OgInt]
    document_id: Optional[OgInt]
    fehler: Optional[OgStr]
    warnungen: Optional[OgStr]
    hash_dedup: Optional[OgStr]
    created_at: Optional[OgDateTime]
    created_by: Optional[OgInt]
    confirmed_at: Optional[OgDateTime]
    confirmed_by: Optional[OgInt]
    supplier: ToOne["ErpSupplier"]

@dataclass
class ErpSupplierCsvMapping:
    """Rail class `ErpSupplierCsvMapping` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    name: Optional[OgStr]
    trennzeichen: Optional[OgStr]
    encoding: Optional[OgStr]
    header_zeile: Optional[OgInt]
    skip_zeilen: Optional[OgInt]
    col_ean: Optional[OgStr]
    col_artikelnr: Optional[OgStr]
    col_bezeichnung: Optional[OgStr]
    col_ek_preis: Optional[OgStr]
    col_mindestmenge: Optional[OgStr]
    col_einheit: Optional[OgStr]
    col_hersteller: Optional[OgStr]
    col_hersteller_anr: Optional[OgStr]
    col_bild_url: Optional[OgStr]
    col_kategorie: Optional[OgStr]
    col_unterkategorie: Optional[OgStr]
    fuzzy_schwelle: Optional[OgInt]
    auto_anlegen: Optional[OgBool]
    auto_kategorie: Optional[OgStr]
    auto_update_url: Optional[OgStr]
    auto_update_aktiv: Optional[OgBool]
    auto_update_auth_type: Optional[OgStr]
    auto_update_auth_user: Optional[OgStr]
    auto_update_auth_pass: Optional[OgStr]
    auto_update_auth_header: Optional[OgStr]
    auto_update_omd_token_url: Optional[OgStr]
    auto_update_omd_client_id: Optional[OgStr]
    auto_update_omd_client_secret: Optional[OgStr]
    auto_update_omd_cred_location: Optional[OgStr]
    auto_update_omd_customer_id: Optional[OgStr]
    auto_update_intervall: Optional[OgInt]
    auto_update_typ: Optional[OgStr]
    auto_update_uhrzeit: Optional[OgStr]
    auto_update_wochentage: Optional[OgStr]
    auto_update_next_run: Optional[OgDateTime]
    auto_inaktiv_setzen: Optional[OgBool]
    auto_update_last_status: Optional[OgStr]
    auto_update_last_error: Optional[OgStr]
    auto_update_last_run: Optional[OgDateTime]
    last_import_at: Optional[OgDateTime]
    last_import_count: Optional[OgInt]
    created_at: Optional[OgDateTime]
    supplier: ToOne["ErpSupplier"]

@dataclass
class ErpStockMovement:
    """Rail class `ErpStockMovement` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    datum: OgDate
    bewegungsart: Optional[OgStr]
    menge: OgFloat
    ek_preis: Optional[OgFloat]
    currency: Optional[OgStr]
    herkunft: Optional[OgStr]
    herkunft_ref_id: Optional[OgInt]
    notiz: Optional[OgStr]
    erfasst_von: Optional[OgInt]
    erfasst_am: Optional[OgDateTime]
    mitarbeiter_id: Optional[OgInt]
    supplier_id: Optional[OgInt]
    gutschrift_nr: Optional[OgStr]
    schadensgrund: Optional[OgStr]
    article: ToOne["Article"]

@dataclass
class ErpAsset:
    """Rail class `ErpAsset` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    inventarnr: Optional[OgStr]
    bezeichnung: OgStr
    anlagenklasse: Optional[OgStr]
    anschaffungsdatum: OgDate
    anschaffungskosten: OgMoney
    currency: Optional[OgStr]
    nutzungsdauer_jahre: Optional[OgInt]
    afa_methode: Optional[OgStr]
    anlagenkonto: Optional[OgStr]
    afa_konto: Optional[OgStr]
    kum_afa: Optional[OgMoney]
    letztes_afa_jahr: Optional[OgInt]
    aktiv: Optional[OgBool]
    abgang_datum: Optional[OgDate]
    abgang_art: Optional[OgStr]
    device_id: Optional[OgInt]
    kostenstelle_id: Optional[OgInt]
    created_at: Optional[OgDateTime]

@dataclass
class ErpBankAccount:
    """Rail class `ErpBankAccount` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    name: Optional[OgStr]
    iban: Optional[OgStr]
    bic: Optional[OgStr]
    sachkonto: Optional[OgStr]
    aktiv: Optional[OgBool]
    kontotyp: Optional[OgStr]
    ist_hauptkonto: Optional[OgBool]
    anfangssaldo: Optional[OgMoney]
    fints_aktiv: Optional[OgBool]
    fints_blz: Optional[OgStr]
    fints_endpoint_url: Optional[OgStr]
    fints_login: Optional[OgStr]
    fints_pin_encrypted: Optional[OgStr]
    fints_tan_methode: Optional[OgStr]
    fints_letzter_abruf: Optional[OgDateTime]
    fints_status: Optional[OgStr]
    fints_letzter_fehler: Optional[OgStr]
    fints_system_id: Optional[OgStr]
    created_at: Optional[OgDateTime]

@dataclass
class ErpBankTransaction:
    """Rail class `ErpBankTransaction` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    bank_account_id: Optional[OgInt]
    buchungsdatum: Optional[OgDate]
    betrag: OgMoney
    verwendungszweck: Optional[OgStr]
    gegenname: Optional[OgStr]
    gegen_iban: Optional[OgStr]
    status: Optional[OgStr]
    match_wo_id: Optional[OgInt]
    match_pi_id: Optional[OgInt]
    match_type: Optional[OgStr]
    match_transfer_tx_id: Optional[OgInt]
    match_sachkonto: Optional[OgStr]
    match_steuersatz: Optional[OgInt]
    import_hash: Optional[OgStr]
    created_at: Optional[OgDateTime]
    document_id: Optional[OgInt]
    match_darlehen_id: Optional[OgInt]
    match_split_data: Optional[OgStr]
    match_abschlag_id: Optional[OgInt]
    match_konto_override: Optional[OgStr]
    hinweis_text: Optional[OgStr]

@dataclass
class ErpDarlehen:
    """Rail class `ErpDarlehen` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    darlehensnr: Optional[OgStr]
    bezeichnung: Optional[OgStr]
    glaeubiger: Optional[OgStr]
    glaeubiger_iban: Optional[OgStr]
    darlehensbetrag_brutto: Optional[OgMoney]
    restschuld: Optional[OgMoney]
    zinssatz: Optional[OgMoney]
    startdatum: Optional[OgDate]
    enddatum_geplant: Optional[OgDate]
    monatsrate_soll: Optional[OgMoney]
    konto_darlehen: Optional[OgStr]
    konto_zinsen: Optional[OgStr]
    aktiv: Optional[OgBool]
    notiz: Optional[OgStr]
    created_at: Optional[OgDateTime]

@dataclass
class ErpAbschlagskonto:
    """Rail class `ErpAbschlagskonto` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    bezeichnung: Optional[OgStr]
    anbieter: Optional[OgStr]
    anbieter_iban: Optional[OgStr]
    vertragsnummer: Optional[OgStr]
    art: Optional[OgStr]
    monatlicher_abschlag: Optional[OgMoney]
    konto_aufwand: Optional[OgStr]
    ust_satz: Optional[OgInt]
    konto_aufwand_2: Optional[OgStr]
    anteil_aufwand_2_proz: Optional[OgInt]
    aktiv: Optional[OgBool]
    notiz: Optional[OgStr]
    created_at: Optional[OgDateTime]

@dataclass
class ErpPaymentRun:
    """Rail class `ErpPaymentRun` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    ausfuehrungsdatum: OgDate
    status: Optional[OgStr]
    summe: Optional[OgMoney]
    anzahl: Optional[OgInt]
    currency: Optional[OgStr]
    xml_exported: Optional[OgBool]
    created_at: Optional[OgDateTime]
    created_by_id: Optional[OgInt]
    bank_account: ToOne["ErpBankAccount"]
    items: ToMany["ErpPaymentRunItem"]

@dataclass
class ErpPaymentRunItem:
    """Rail class `ErpPaymentRunItem` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    run_id: OgInt
    purchase_invoice_id: Optional[OgInt]
    empfaenger: OgStr
    iban: OgStr
    bic: Optional[OgStr]
    betrag: OgMoney
    currency: Optional[OgStr]
    verwendungszweck: Optional[OgStr]

@dataclass
class ErpVatReturn:
    """Rail class `ErpVatReturn` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    jahr: OgInt
    monat: OgInt
    kz81: Optional[OgMoney]
    kz86: Optional[OgMoney]
    kz66: Optional[OgMoney]
    ust_19: Optional[OgMoney]
    ust_7: Optional[OgMoney]
    zahllast: Optional[OgMoney]
    status: Optional[OgStr]
    berechnet_am: Optional[OgDateTime]
    created_at: Optional[OgDateTime]

@dataclass
class ErpPreisSchema:
    """Rail class `ErpPreisSchema` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    name: OgStr
    beschreibung: Optional[OgStr]
    fix_aufschlag: Optional[OgMoney]
    rundung_typ: Optional[OgStr]
    mindestmarge_pct: Optional[OgMoney]
    auto_neuberechnung: Optional[OgBool]
    ist_default: Optional[OgBool]
    aktiv: Optional[OgBool]
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]
    spannen: ToMany["ErpPreisSpanne"]

@dataclass
class ErpPreisSpanne:
    """Rail class `ErpPreisSpanne` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    schema_id: OgInt
    preis_von: OgMoney
    preis_bis: Optional[OgMoney]
    aufschlag_pct: OgMoney
    sort_order: Optional[OgInt]

@dataclass
class ErpPreisGruppe:
    """Rail class `ErpPreisGruppe` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    name: OgStr
    rabatt_pct: Optional[OgMoney]
    schema_id: Optional[OgInt]
    ist_default: Optional[OgBool]
    sort_order: Optional[OgInt]
    aktiv: Optional[OgBool]
    created_at: Optional[OgDateTime]
    kunden: ToMany["Customer"]

@dataclass
class ErpLohnPeriode:
    """Rail class `ErpLohnPeriode` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    jahr: OgInt
    monat: OgInt
    brutto_loehne: Optional[OgMoney]
    ag_sv_anteile: Optional[OgMoney]
    an_sv_anteile: Optional[OgMoney]
    lohnsteuer_gesamt: Optional[OgMoney]
    currency: Optional[OgStr]
    notiz: Optional[OgStr]
    gebucht_am: Optional[OgDateTime]
    gebucht_von: Optional[OgInt]
    erfasst_am: Optional[OgDateTime]
    erfasst_von: Optional[OgInt]

@dataclass
class DashboardDismissal:
    """Rail class `DashboardDismissal` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    user_id: OgInt
    item_key: OgStr
    created_at: Optional[OgDateTime]

@dataclass
class CustomerRemoteDevice:
    """Rail class `CustomerRemoteDevice` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    provider: OgStr
    label: Optional[OgStr]
    device_id: OgStr
    notizen: Optional[OgStr]
    created_at: Optional[OgDateTime]
    customer: ToOne["Customer"]

@dataclass
class CrmCompany:
    """Rail class `CrmCompany` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    rechtsform: Optional[OgStr]
    branche: Optional[OgStr]
    website: Optional[OgStr]
    telefon: Optional[OgStr]
    email: Optional[OgStr]
    strasse: Optional[OgStr]
    plz: Optional[OgStr]
    ort: Optional[OgStr]
    land: Optional[OgStr]
    ust_id: Optional[OgStr]
    customer_id: Optional[OgInt]
    inhaber_user_id: Optional[OgInt]
    notizen: Optional[OgStr]
    tags: Optional[OgStr]
    aktiv: Optional[OgBool]
    geloescht_am: Optional[OgDateTime]
    erstellt_am: Optional[OgDateTime]
    geaendert_am: Optional[OgDateTime]

@dataclass
class CrmContact:
    """Rail class `CrmContact` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    company_id: Optional[OgInt]
    customer_id: Optional[OgInt]
    anrede: Optional[OgStr]
    vorname: Optional[OgStr]
    nachname: OgStr
    position: Optional[OgStr]
    abteilung: Optional[OgStr]
    email: Optional[OgStr]
    telefon: Optional[OgStr]
    mobil: Optional[OgStr]
    ist_hauptkontakt: Optional[OgBool]
    quelle: Optional[OgStr]
    notizen: Optional[OgStr]
    tags: Optional[OgStr]
    aktiv: Optional[OgBool]
    geloescht_am: Optional[OgDateTime]
    erstellt_am: Optional[OgDateTime]
    geaendert_am: Optional[OgDateTime]
    carddav_uid: Optional[OgStr]
    carddav_href: Optional[OgStr]
    carddav_etag: Optional[OgStr]
    carddav_synced_at: Optional[OgDateTime]

@dataclass
class CrmPipeline:
    """Rail class `CrmPipeline` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    ist_default: Optional[OgBool]
    sort_order: Optional[OgInt]
    aktiv: Optional[OgBool]
    geloescht_am: Optional[OgDateTime]
    erstellt_am: Optional[OgDateTime]

@dataclass
class CrmStage:
    """Rail class `CrmStage` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    pipeline_id: OgInt
    name: OgStr
    sort_order: Optional[OgInt]
    wahrscheinlichkeit: Optional[OgInt]
    ist_gewonnen: Optional[OgBool]
    ist_verloren: Optional[OgBool]
    farbe: Optional[OgStr]
    aktiv: Optional[OgBool]
    geloescht_am: Optional[OgDateTime]

@dataclass
class CrmLead:
    """Rail class `CrmLead` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    titel: OgStr
    pipeline_id: OgInt
    stage_id: OgInt
    contact_id: Optional[OgInt]
    company_id: Optional[OgInt]
    customer_id: Optional[OgInt]
    wert_netto: Optional[OgMoney]
    waehrung: Optional[OgStr]
    wahrscheinlichkeit: Optional[OgInt]
    quelle: Optional[OgStr]
    zustaendig_user_id: Optional[OgInt]
    erwartetes_datum: Optional[OgDate]
    status: Optional[OgStr]
    verloren_grund: Optional[OgStr]
    source_cold_lead_id: Optional[OgInt]
    notizen: Optional[OgStr]
    tags: Optional[OgStr]
    category_id: Optional[OgInt]
    aktiv: Optional[OgBool]
    geloescht_am: Optional[OgDateTime]
    erstellt_am: Optional[OgDateTime]
    geaendert_am: Optional[OgDateTime]

@dataclass
class CrmCategory:
    """Rail class `CrmCategory` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    farbe: Optional[OgStr]
    sort_order: Optional[OgInt]
    aktiv: Optional[OgBool]
    geloescht_am: Optional[OgDateTime]
    erstellt_am: Optional[OgDateTime]

@dataclass
class CrmActivity:
    """Rail class `CrmActivity` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    entity_type: OgStr
    entity_id: OgInt
    typ: OgStr
    betreff: Optional[OgStr]
    inhalt: Optional[OgStr]
    richtung: Optional[OgStr]
    ref_table: Optional[OgStr]
    ref_id: Optional[OgInt]
    user_id: Optional[OgInt]
    zeitpunkt: Optional[OgDateTime]
    geloescht_am: Optional[OgDateTime]
    erstellt_am: Optional[OgDateTime]

@dataclass
class CrmTask:
    """Rail class `CrmTask` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    titel: OgStr
    beschreibung: Optional[OgStr]
    entity_type: Optional[OgStr]
    entity_id: Optional[OgInt]
    faellig_am: Optional[OgDateTime]
    erinnerung_am: Optional[OgDateTime]
    prioritaet: Optional[OgStr]
    status: Optional[OgStr]
    zustaendig_user_id: Optional[OgInt]
    erledigt_am: Optional[OgDateTime]
    geloescht_am: Optional[OgDateTime]
    erstellt_am: Optional[OgDateTime]
    geaendert_am: Optional[OgDateTime]

@dataclass
class CrmDocument:
    """Rail class `CrmDocument` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    titel: OgStr
    vorlage_id: Optional[OgInt]
    entity_type: Optional[OgStr]
    entity_id: Optional[OgInt]
    customer_id: Optional[OgInt]
    storage_key: Optional[OgStr]
    mime_type: Optional[OgStr]
    format: Optional[OgStr]
    version: Optional[OgInt]
    erp_document_id: Optional[OgInt]
    status: Optional[OgStr]
    erstellt_von: Optional[OgInt]
    folder_id: Optional[OgInt]
    aktiv: Optional[OgBool]
    geloescht_am: Optional[OgDateTime]
    erstellt_am: Optional[OgDateTime]
    geaendert_am: Optional[OgDateTime]

@dataclass
class CrmDocumentFolder:
    """Rail class `CrmDocumentFolder` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    entity_type: OgStr
    entity_id: OgInt
    parent_id: Optional[OgInt]
    name: OgStr
    sort_order: Optional[OgInt]
    geloescht_am: Optional[OgDateTime]
    erstellt_am: Optional[OgDateTime]

@dataclass
class CrmDocTemplate:
    """Rail class `CrmDocTemplate` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    name: OgStr
    format: Optional[OgStr]
    storage_key: Optional[OgStr]
    beschreibung: Optional[OgStr]
    kategorie: Optional[OgStr]
    aktiv: Optional[OgBool]
    geloescht_am: Optional[OgDateTime]
    erstellt_am: Optional[OgDateTime]

@dataclass
class CrmDocumentLink:
    """Rail class `CrmDocumentLink` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    erp_document_id: OgInt
    entity_type: OgStr
    entity_id: OgInt
    zugeordnet_von_user_id: Optional[OgInt]
    zugeordnet_am: Optional[OgDateTime]
    quelle: Optional[OgStr]
    geloescht_am: Optional[OgDateTime]
    erstellt_am: Optional[OgDateTime]

@dataclass
class RentedServerEvent:
    """Rail class `RentedServerEvent` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    ts: OgDateTime
    kind: OgStr
    level: Optional[OgStr]
    message: Optional[OgStr]
    payload: Optional[OgStr]
    server: ToOne["RentedServer"]

@dataclass
class ShopProduct:
    """Rail class `ShopProduct` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    sku: Optional[OgStr]
    name: OgStr
    slug: OgStr
    short_desc: Optional[OgStr]
    long_desc: Optional[OgStr]
    product_type: OgStr
    price_cents: OgInt
    currency: OgStr
    vat_rate: OgMoney
    stock_qty: OgInt
    weight_grams: Optional[OgInt]
    saas_meta_json: Optional[OgScalar]
    digital_path: Optional[OgStr]
    image_path: Optional[OgStr]
    gallery_json: Optional[OgScalar]
    status: OgStr
    sort: OgInt
    meta_title: Optional[OgStr]
    meta_description: Optional[OgStr]
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]

@dataclass
class ShopCategory:
    """Rail class `ShopCategory` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    name: OgStr
    slug: OgStr
    description: Optional[OgStr]
    parent_id: Optional[OgInt]
    sort: OgInt
    image_path: Optional[OgStr]
    status: OgStr
    created_at: Optional[OgDateTime]
    children: ToMany["ShopCategory"]

@dataclass
class ShopOrder:
    """Rail class `ShopOrder` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    order_number: OgStr
    confirmation_token: OgStr
    customer_email: OgStr
    customer_name: OgStr
    customer_phone: Optional[OgStr]
    woa_customer_id: Optional[OgInt]
    billing_address_json: OgScalar
    shipping_address_json: Optional[OgScalar]
    has_shipping: OgBool
    subtotal_cents: OgInt
    vat_cents: OgInt
    shipping_cost_cents: OgInt
    total_cents: OgInt
    currency: OgStr
    payment_provider: OgStr
    payment_status: OgStr
    payment_provider_ref: Optional[OgStr]
    payment_meta_json: Optional[OgScalar]
    order_status: OgStr
    notes_customer: Optional[OgStr]
    notes_internal: Optional[OgStr]
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]
    paid_at: Optional[OgDateTime]
    shipped_at: Optional[OgDateTime]
    items: ToMany["ShopOrderItem"]

@dataclass
class ShopOrderItem:
    """Rail class `ShopOrderItem` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    order_id: OgInt
    sku: Optional[OgStr]
    name: OgStr
    product_type: OgStr
    qty: OgInt
    unit_price_cents: OgInt
    vat_rate: OgMoney
    line_total_cents: OgInt
    meta_json: Optional[OgScalar]
    product: ToOne["ShopProduct"]

@dataclass
class ShopCartSession:
    """Rail class `ShopCartSession` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    cart_key: OgStr
    tenant_id: Optional[OgInt]
    items_json: Optional[OgScalar]
    customer_email: Optional[OgStr]
    expires_at: OgDateTime
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]

@dataclass
class ShopPage:
    """Rail class `ShopPage` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    slug: OgStr
    title: OgStr
    content_html: Optional[OgStr]
    meta_title: Optional[OgStr]
    meta_description: Optional[OgStr]
    status: OgStr
    sort: OgInt
    is_system: OgBool
    show_in_footer: OgBool
    show_in_header: OgBool
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]

@dataclass
class ShopShippingMethod:
    """Rail class `ShopShippingMethod` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    code: OgStr
    name: OgStr
    description: Optional[OgStr]
    icon: Optional[OgStr]
    price_cents: OgInt
    free_threshold_cents: Optional[OgInt]
    weight_max_grams: Optional[OgInt]
    delivery_time_min_days: Optional[OgInt]
    delivery_time_max_days: Optional[OgInt]
    countries_allowed: Optional[OgStr]
    active: OgBool
    sort: OgInt
    created_at: Optional[OgDateTime]

@dataclass
class ShopPaymentMethod:
    """Rail class `ShopPaymentMethod` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    code: OgStr
    label: OgStr
    description: Optional[OgStr]
    icon: Optional[OgStr]
    fee_cents: OgInt
    fee_percent: OgMoney
    provider: OgStr
    min_total_cents: Optional[OgInt]
    max_total_cents: Optional[OgInt]
    requires_address: OgBool
    active: OgBool
    is_default: OgBool
    sort: OgInt
    created_at: Optional[OgDateTime]

@dataclass
class ShopCustomer:
    """Rail class `ShopCustomer` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    email: OgStr
    password_hash: OgStr
    first_name: Optional[OgStr]
    last_name: Optional[OgStr]
    company: Optional[OgStr]
    phone: Optional[OgStr]
    vat_id: Optional[OgStr]
    billing_address_json: Optional[OgScalar]
    shipping_address_json: Optional[OgScalar]
    is_b2b: OgBool
    status: OgStr
    email_verified: OgBool
    verify_token: Optional[OgStr]
    verify_token_expires: Optional[OgDateTime]
    reset_token: Optional[OgStr]
    reset_token_expires: Optional[OgDateTime]
    failed_attempts: OgInt
    locked_until: Optional[OgDateTime]
    last_login_at: Optional[OgDateTime]
    last_login_ip: Optional[OgStr]
    accepts_marketing: OgBool
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]

@dataclass
class ShopPaymentConfig:
    """Rail class `ShopPaymentConfig` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    provider: OgStr
    mode: OgStr
    public_key: Optional[OgStr]
    secret_key: Optional[OgStr]
    webhook_secret: Optional[OgStr]
    extra_json: Optional[OgScalar]
    active: OgBool
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]

@dataclass
class ShopSaasConfig:
    """Rail class `ShopSaasConfig` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    order_item_id: Optional[OgInt]
    article_id: OgInt
    dns_mode: OgStr
    dns_notes: Optional[OgStr]
    extra_users: OgInt
    extra_users_price_cents_each: OgInt
    addon_article_ids: Optional[OgScalar]
    wants_training: OgBool
    wants_test_data: OgBool
    seed_articles: OgBool
    branche: Optional[OgStr]
    desired_slug: Optional[OgStr]
    admin_username: Optional[OgStr]
    notes_for_admin: Optional[OgStr]
    setup_service: OgBool
    setup_data_json: Optional[OgScalar]
    chosen_billing_period: Optional[OgStr]
    payment_mode: OgStr
    payment_iban: Optional[OgStr]
    payment_bic: Optional[OgStr]
    payment_holder: Optional[OgStr]
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]
    order: ToOne["ShopOrder"]

@dataclass
class ShopProductReview:
    """Rail class `ShopProductReview` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: Optional[OgInt]
    article_id: OgInt
    rating: OgInt
    title: Optional[OgStr]
    comment: Optional[OgStr]
    status: OgStr
    created_at: Optional[OgDateTime]
    updated_at: Optional[OgDateTime]
    customer: ToOne["ShopCustomer"]

