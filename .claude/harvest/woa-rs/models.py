"""WoA sink-in substrate v1 — generated, do not hand-edit.

Source: /home/user/WoA/models.py (READ-ONLY corpus, 139 `db.Model` classes harvested)
Pipeline: ruff_sqlalchemy_spo::extract_file -> ogar-from-ruff::lift_model_graph_sqlalchemy
          -> ogar-from-ruff::mint::compile_graph_sqlalchemy::<WoaPort>
          -> ogar-from-ruff::emit::emit_python
Metrics: 139 classes, 1961 attributes, 107 associations, 6 aliased (WOA_ALIASES convergence pin) / 133 bootstrap (classid 0x0000_0003).
TimesheetActivity -> classid 0x01030003 (concept 0x0103, app 0x0003)
"""

from dataclasses import dataclass
from typing import ClassVar

from ogar_sdk import OgScalar, ToOne, ToMany


@dataclass
class Tenant:
    """Rail class `Tenant` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    name: OgStr
    slug: OgStr
    max_users: OgInt
    aktiv: OgBool
    logo_path: OgStr
    logo_mail_path: OgStr
    created_at: OgDateTime
    branche: OgStr
    is_test: OgBool
    is_anbieter: OgBool
    mod_tresor: OgBool
    mod_stundenzettel: OgBool
    mod_multitimer: OgBool
    mod_fahrtenbuch: OgBool
    mod_wiedervorlage: OgBool
    mod_notizbuch: OgBool
    mod_wartung: OgBool
    mod_abo: OgBool
    mod_inventar: OgBool
    mod_abnahme: OgBool
    mod_gobd: OgBool
    mod_referral: OgBool
    mod_kaltakquise: OgBool
    mod_woa_service: OgBool
    mod_erp: OgBool
    mod_dms: OgBool
    mod_rustdesk: OgBool
    mod_rustdesk_server: OgBool
    erp_gobd_festschreibung: OgBool
    erp_mod_stammdaten: OgBool
    erp_mod_fibu: OgBool
    erp_mod_bank: OgBool
    erp_mod_steuer: OgBool
    erp_mod_lager: OgBool
    erp_mod_dms: OgBool
    erp_mod_reporting: OgBool
    erp_mod_pos: OgBool
    erp_mod_zugferd: OgBool
    erp_mod_lohn: OgBool
    erp_mod_shop: OgBool
    erp_mod_crm: OgBool
    erp_mod_stammdaten_gesperrt: OgBool
    erp_mod_fibu_gesperrt: OgBool
    erp_mod_bank_gesperrt: OgBool
    erp_mod_steuer_gesperrt: OgBool
    erp_mod_lager_gesperrt: OgBool
    erp_mod_dms_gesperrt: OgBool
    erp_mod_reporting_gesperrt: OgBool
    erp_mod_pos_gesperrt: OgBool
    erp_mod_zugferd_gesperrt: OgBool
    erp_mod_crm_gesperrt: OgBool
    erp_mod_lohn_gesperrt: OgBool
    erp_mod_shop_gesperrt: OgBool

@dataclass
class User:
    """Rail class `User` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    username: OgStr
    password_hash: OgStr
    firstname: OgStr
    lastname: OgStr
    email: OgStr
    phone: OgStr
    ma_rabatt: OgFloat
    is_admin: OgBool
    is_superadmin: OgBool
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
    failed_attempts: OgInt
    locked_until: OgDateTime
    scan_import_mode: OgStr
    scan_import_username: OgStr
    scan_import_pw_set_at: OgDateTime
    scan_import_host: OgStr
    scan_import_port: OgInt
    vpn_enabled: OgBool
    vpn_ip: OgStr
    vpn_pubkey: OgStr
    vpn_created_at: OgDateTime
    samba_enabled: OgBool
    samba_pw_set_at: OgDateTime
    created_at: OgDateTime
    tenant: ToOne["Tenant"]
    tresor_customer: ToOne["Customer"]

@dataclass
class Customer:
    """Rail class `Customer` — classid 0x02040003 (concept 0x0204, app 0x0003)."""
    CLASSID: ClassVar[int] = 0x02040003
    id: OgInt
    tenant_id: OgInt
    kdnr: OgStr
    quick_token: OgStr
    quick_token_ts: OgStr
    firma: OgStr
    anrede: OgStr
    vorname: OgStr
    nachname: OgStr
    mail_anrede: OgStr
    strasse: OgStr
    adresszusatz: OgStr
    plz: OgStr
    ort: OgStr
    email: OgStr
    telefon: OgStr
    tresor_pw_hash: OgStr
    tresor_pw_set_at: OgDateTime
    tresor_pw_failed: OgInt
    tresor_pw_locked_until: OgDateTime
    zahlungsziel: OgInt
    skonto_prozent: OgFloat
    skonto_tage: OgInt
    stundensatz: OgFloat
    fahrt_km: OgFloat
    fahrt_kosten: OgFloat
    notizen: OgStr
    aktiv: OgBool
    kundentyp: OgStr
    referral_code: OgStr
    sepa_iban: OgStr
    sepa_bic: OgStr
    sepa_kontoinhaber: OgStr
    sepa_mandat_ref: OgStr
    sepa_mandat_datum: OgDate
    sepa_mandat_typ: OgStr
    sepa_mandat_status: OgStr
    sepa_letzte_lastschrift: OgDate
    sepa_pre_notification_tage: OgInt
    preis_gruppe_id: OgInt
    created_at: OgDateTime
    workorders: ToMany["WorkOrder"]

@dataclass
class Project:
    """Rail class `Project` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    projekt_nr: OgStr
    beschreibung: OgStr
    status: OgStr
    sort_order: OgInt
    erstellt_am: OgDateTime
    erstellt_von_id: OgInt
    abgeschlossen_am: OgDateTime
    customer: ToOne["Customer"]
    notes: ToOne["ProjectNote"]

@dataclass
class ProjectNote:
    """Rail class `ProjectNote` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    titel: OgStr
    inhalt_text: OgScalar
    inhalt_zeichnung: OgScalar
    erstellt_am: OgDateTime
    erstellt_von_id: OgInt
    project: ToOne["Project"]

@dataclass
class ErpArticleCategory:
    """Rail class `ErpArticleCategory` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    icon: OgStr
    sort_order: OgInt
    created_at: OgDateTime
    parent: ToMany["ErpArticleCategory"]

@dataclass
class ErpStorageLocation:
    """Rail class `ErpStorageLocation` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    kuerzel: OgStr
    icon: OgStr
    beschreibung: OgStr
    sort_order: OgInt
    aktiv: OgBool
    created_at: OgDateTime
    parent: ToMany["ErpStorageLocation"]

@dataclass
class Article:
    """Rail class `Article` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    artikelnr: OgStr
    ean: OgStr
    beschreibung: OgStr
    kategorie: OgStr
    category_id: OgInt
    storage_location_id: OgInt
    einheit: OgStr
    hersteller: OgStr
    hersteller_anr: OgStr
    bild_url: OgStr
    bestand: OgFloat
    mindestbestand: OgFloat
    preis_netto: OgMoney
    ek_preis: OgMoney
    mwst_satz: OgFloat
    tax_rate_id: OgInt
    lieferant: OgStr
    lieferant_anr: OgStr
    notizen: OgStr
    typ: OgStr
    aktiv: OgBool
    dauer_minuten: OgInt
    preis_schema_id: OgInt
    vk_preis_manuell: OgBool
    listenpreis: OgMoney
    uvp: OgMoney
    gewicht_kg: OgMoney
    herkunftsland: OgStr
    zolltarifnr: OgStr
    langbeschreibung: OgStr
    matchcode: OgStr
    warengruppe: OgStr
    warengruppe_nr: OgStr
    gefahrgut: OgBool
    gefahrgut_un_nr: OgStr
    gefahrgut_klasse: OgStr
    auslaufartikel: OgBool
    auslaufdatum: OgDate
    deeplink: OgStr
    shop_active: OgBool
    shop_product_type: OgStr
    shop_category_id: OgInt
    shop_long_desc_html: OgStr
    shop_saas_meta_json: OgScalar
    shop_saas_package_id: OgInt
    shop_digital_path: OgStr
    shop_meta_title: OgStr
    shop_meta_description: OgStr
    shop_slug: OgStr
    shop_payment_methods_csv: OgStr
    shop_free_shipping: OgBool
    shop_extra_user_price_cents: OgInt
    shop_addon_article_ids: OgScalar
    shop_included_users: OgInt
    omd_tax_code: OgInt

@dataclass
class WorkOrder:
    """Rail class `WorkOrder` — classid 0x02020003 (concept 0x0202, app 0x0003)."""
    CLASSID: ClassVar[int] = 0x02020003
    id: OgInt
    tenant_id: OgInt
    customer_id: OgInt
    created_by: OgInt
    doc_type: OgStr
    status: OgStr
    angebot_nr: OgStr
    auftrags_nr: OgStr
    workorder_nr: OgStr
    rechnung_nr: OgStr
    gutschrift_nr: OgStr
    sammelrechnung_id: OgInt
    datum: OgDate
    zeit_start: OgStr
    zeit_ende: OgStr
    anfahrten: OgFloat
    mitarbeiter: OgFloat
    pause_h: OgFloat
    zusatz_h: OgFloat
    betreff: OgStr
    notizen: OgStr
    intern_notizen: OgStr
    bezahlt: OgBool
    bezahlt_am: OgDate
    bezahlt_betrag: OgMoney
    mahnstufe: OgInt
    letzte_mahnung: OgDate
    erfuellung_bis: OgDate
    unterschrift: OgStr
    signed_at: OgDateTime
    signed_ip: OgStr
    signed_user_agent: OgStr
    zahlungsart: OgStr
    anzahlung_prozent: OgFloat
    anzahlung_betrag: OgFloat
    anzahlung_bezahlt: OgBool
    anzahlung_bezahlt_am: OgDate
    kleinunternehmer_snapshot: OgBool
    zahlungsziel_tage_snapshot: OgInt
    gesamt_rabatt_prozent: OgFloat
    gesamt_rabatt_betrag: OgFloat
    skonto_prozent_snapshot: OgFloat
    skonto_tage_snapshot: OgInt
    skonto_ausweisen: OgBool
    skonto_aufschlag: OgBool
    skonto_aufschlag_faktor: OgFloat
    created_at: OgDateTime
    updated_at: OgDateTime
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
    article_id: OgInt
    sort_order: OgInt
    pos_typ: OgStr
    beschreibung: OgStr
    menge: OgFloat
    einheit: OgStr
    einzelpreis: OgMoney
    mwst_satz: OgFloat
    tax_rate_id: OgInt
    versteckt: OgBool
    is_optional: OgBool
    customer_accepted_at: OgDateTime
    rabatt_prozent: OgFloat
    einzelpreis_vor_skonto: OgMoney

@dataclass
class Activity:
    """Rail class `Activity` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    workorder_id: OgInt
    geraet: OgStr
    beschreibung: OgStr
    logbuch: OgBool
    intern: OgBool
    created_at: OgDateTime
    acceptance_items: ToOne["AcceptanceItem"]

@dataclass
class Picture:
    """Rail class `Picture` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    workorder_id: OgInt
    dateiname: OgStr
    beschreibung: OgStr
    logbuch: OgBool
    an_kunde: OgBool
    created_at: OgDateTime

@dataclass
class Document:
    """Rail class `Document` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    dateiname: OgStr
    original_name: OgStr
    beschreibung: OgStr
    mime_type: OgStr
    size_bytes: OgInt
    an_kunde: OgBool
    uploaded_by: OgInt
    uploaded_at: OgDateTime
    customer: ToOne["Customer"]
    workorder: ToOne["WorkOrder"]

@dataclass
class AcceptanceProtocol:
    """Rail class `AcceptanceProtocol` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    sequenz_nr: OgInt
    vorgaenger_id: OgInt
    aktiv: OgBool
    gesamt_abgenommen: OgBool
    abnahme_datum: OgDate
    abnahme_ort: OgStr
    bemerkungen: OgStr
    nachbesserungstermin: OgDate
    unterschrift_kunde: OgStr
    unterschrieben_am: OgDateTime
    unterschrieben_von: OgStr
    erstellt_am: OgDateTime
    erstellt_von_id: OgInt
    workorder: ToOne["WorkOrder"]
    items: ToOne["AcceptanceItem"]
    defects: ToOne["AcceptanceDefect"]

@dataclass
class AcceptanceItem:
    """Rail class `AcceptanceItem` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    abgenommen: OgBool
    bemerkung: OgStr
    bezeichnung: OgStr
    status: OgStr
    sort_order: OgInt
    protocol: ToOne["AcceptanceProtocol"]
    activity: ToOne["Activity"]

@dataclass
class AcceptanceDefect:
    """Rail class `AcceptanceDefect` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    beschreibung: OgStr
    erfasst_am: OgDateTime
    nachbesserung_bis: OgDate
    behoben: OgBool
    behoben_am: OgDateTime
    intern_status: OgStr
    intern_status_am: OgDateTime
    protocol: ToOne["AcceptanceProtocol"]

@dataclass
class AcceptanceTemplate:
    """Rail class `AcceptanceTemplate` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    is_default: OgBool
    aktiv: OgBool
    erstellt_am: OgDateTime
    items: ToOne["AcceptanceTemplateItem"]

@dataclass
class AcceptanceTemplateItem:
    """Rail class `AcceptanceTemplateItem` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    bezeichnung: OgStr
    sort_order: OgInt
    template: ToOne["AcceptanceTemplate"]

@dataclass
class HistoryEntry:
    """Rail class `HistoryEntry` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    workorder_id: OgInt
    aktion: OgStr
    details: OgStr
    created_at: OgDateTime
    user: ToOne["User"]

@dataclass
class LogbookEntry:
    """Rail class `LogbookEntry` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    workorder_id: OgInt
    datum: OgDate
    abfahrt: OgStr
    ankunft: OgStr
    rueckfahrt: OgStr
    zurueck: OgStr
    start_km: OgFloat
    ende_km: OgFloat
    route: OgStr
    zweck: OgStr
    fahrzeug: OgStr
    privat_anteil: OgFloat
    created_at: OgDateTime
    user: ToOne["User"]
    customer: ToOne["Customer"]

@dataclass
class NumberSequence:
    """Rail class `NumberSequence` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    prefix: OgStr
    current: OgInt
    padding: OgInt

@dataclass
class Setting:
    """Rail class `Setting` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    key: OgStr
    value: OgStr
    label: OgStr
    override_master: OgInt

@dataclass
class CustomerPortalUser:
    """Rail class `CustomerPortalUser` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    username: OgStr
    password_hash: OgStr
    aktiv: OgBool
    must_change_pw: OgBool
    last_login: OgDateTime
    created_at: OgDateTime
    failed_attempts: OgInt
    locked_until: OgDateTime
    customer: ToOne["Customer"]

@dataclass
class PasswordEntry:
    """Rail class `PasswordEntry` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    created_by: OgInt
    gruppe: OgStr
    titel: OgStr
    benutzername: OgStr
    passwort_enc: OgStr
    url: OgStr
    notizen_enc: OgStr
    icon: OgStr
    aktiv: OgBool
    keepass_uid: OgStr
    created_at: OgDateTime
    updated_at: OgDateTime
    customer: ToOne["Customer"]
    creator: ToOne["User"]

@dataclass
class TimeSheet:
    """Rail class `TimeSheet` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    source: OgStr
    datum: OgDate
    minuten: OgInt
    erfasst_von: OgStr
    startzeit: OgScalar
    endzeit: OgScalar
    beschreibung: OgStr
    timer_start: OgDateTime
    timer_paused_at: OgDateTime
    abgerechnet: OgBool
    created_at: OgDateTime
    updated_at: OgDateTime
    customer: ToOne["Customer"]
    user: ToOne["User"]

@dataclass
class TaxReserve:
    """Rail class `TaxReserve` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    jahr: OgInt
    monat: OgInt
    quartal: OgInt
    typ: OgStr
    erledigt: OgBool

@dataclass
class TimesheetActivity:
    """Rail class `TimesheetActivity` — classid 0x01030003 (concept 0x0103, app 0x0003)."""
    CLASSID: ClassVar[int] = 0x01030003
    id: OgInt
    beschreibung: OgStr
    created_at: OgDateTime
    timesheet: ToOne["TimeSheet"]

@dataclass
class Reminder:
    """Rail class `Reminder` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    titel: OgStr
    beschreibung: OgStr
    faellig_am: OgDate
    prioritaet: OgStr
    erledigt: OgBool
    erledigt_am: OgDateTime
    erstellt_am: OgDateTime
    zeit_von: OgStr
    zeit_bis: OgStr
    termin_typ: OgStr
    wiederkehrend: OgBool
    intervall: OgStr
    intervall_tage: OgStr
    intervall_tag: OgInt
    intervall_monat: OgInt
    intervall_alle: OgInt
    token_cancel: OgStr
    cancelled_at: OgDateTime
    cancelled_ip: OgStr
    crm_lead_id: OgInt
    crm_contact_id: OgInt
    crm_company_id: OgInt
    crm_task_id: OgInt
    project: ToOne["Project"]
    user: ToOne["User"]
    customer: ToOne["Customer"]
    workorder: ToOne["WorkOrder"]

@dataclass
class MaintenanceContract:
    """Rail class `MaintenanceContract` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    titel: OgStr
    beschreibung: OgStr
    intervall: OgStr
    preis_netto: OgFloat
    mwst_satz: OgFloat
    beginn: OgDate
    ende: OgDate
    letzte_wartung: OgDate
    naechste_wartung: OgDate
    auto_rechnung: OgBool
    aktiv: OgBool
    notizen: OgStr
    created_at: OgDateTime
    customer: ToMany["Customer"]

@dataclass
class RecurringInvoice:
    """Rail class `RecurringInvoice` — classid 0x02020003 (concept 0x0202, app 0x0003)."""
    CLASSID: ClassVar[int] = 0x02020003
    id: OgInt
    tenant_id: OgInt
    titel: OgStr
    beschreibung: OgStr
    intervall: OgStr
    preis_netto: OgFloat
    mwst_satz: OgFloat
    naechste_ausfuehrung: OgDate
    letzte_ausfuehrung: OgDate
    auto_versand: OgBool
    aktiv: OgBool
    notizen: OgStr
    created_at: OgDateTime
    customer: ToOne["Customer"]
    contract: ToOne["MaintenanceContract"]

@dataclass
class Device:
    """Rail class `Device` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    kategorie: OgStr
    hersteller: OgStr
    modell: OgStr
    seriennummer: OgStr
    hostname: OgStr
    ip_adresse: OgStr
    mac_adresse: OgStr
    standort: OgStr
    kaufdatum: OgDate
    garantie_bis: OgDate
    firmware: OgStr
    zugangsdaten: OgStr
    notizen: OgStr
    letzte_wartung: OgDate
    status: OgStr
    aktiv: OgBool
    created_at: OgDateTime
    updated_at: OgDateTime
    customer: ToMany["Customer"]

@dataclass
class KummerkastenEntry:
    """Rail class `KummerkastenEntry` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    source: OgStr
    typ: OgStr
    titel: OgStr
    beschreibung: OgStr
    prioritaet: OgStr
    status: OgStr
    admin_kommentar: OgStr
    erstellt_am: OgDateTime
    aktualisiert_am: OgDateTime
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
    last_seen_at: OgDateTime

@dataclass
class ReferralLog:
    """Rail class `ReferralLog` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    referral_code: OgStr
    empfaenger_email: OgStr
    empfaenger_name: OgStr
    versendet_am: OgDateTime
    versendet_von: OgInt
    status: OgStr
    converted_at: OgDateTime
    converted_manually: OgBool
    converted_note: OgStr
    proposed_by: OgInt
    proposed_at: OgDateTime
    converted_tenant_id: OgInt
    customer: ToOne["Customer"]
    tenant: ToOne["Tenant"]

@dataclass
class ColdLead:
    """Rail class `ColdLead` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    firma: OgStr
    ansprechpartner: OgStr
    branche: OgStr
    strasse: OgStr
    plz: OgStr
    ort: OgStr
    land: OgStr
    telefon: OgStr
    mobil: OgStr
    email: OgStr
    webseite: OgStr
    quelle: OgStr
    briefanrede: OgStr
    mitarbeiterzahl: OgStr
    notiz_kurz: OgStr
    newsletter_sperre: OgBool
    newsletter_sperre_grund: OgStr
    newsletter_sperre_am: OgDateTime
    source_customer_id: OgInt
    status: OgStr
    wiedervorlage_am: OgDate
    notizen: OgStr
    converted_customer_id: OgInt
    converted_at: OgDateTime
    created_at: OgDateTime
    created_by: OgInt
    updated_at: OgDateTime
    activities: ToMany["ColdLeadActivity"]

@dataclass
class ColdLeadActivity:
    """Rail class `ColdLeadActivity` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    lead_id: OgInt
    typ: OgStr
    text: OgStr
    mail_subject: OgStr
    mail_to: OgStr
    status_from: OgStr
    status_to: OgStr
    created_at: OgDateTime
    created_by: OgInt

@dataclass
class ColdCampaign:
    """Rail class `ColdCampaign` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    beschreibung: OgStr
    status: OgStr
    mail_subject: OgStr
    mail_template_html: OgStr
    created_at: OgDateTime
    created_by: OgInt
    updated_at: OgDateTime
    leads: ToMany["ColdCampaignLead"]

@dataclass
class ColdCampaignLead:
    """Rail class `ColdCampaignLead` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    campaign_id: OgInt
    sent_at: OgDateTime
    sent_status: OgStr
    sent_error: OgStr
    added_at: OgDateTime
    added_by: OgInt
    lead: ToOne["ColdLead"]

@dataclass
class ServicePackage:
    """Rail class `ServicePackage` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    beschreibung: OgStr
    rechnungs_titel: OgStr
    rechnungs_text: OgStr
    preis_monthly: OgFloat
    preis_quarterly: OgFloat
    preis_half_yearly: OgFloat
    preis_yearly: OgFloat
    mwst_satz: OgFloat
    free_months_default: OgInt
    aktiv: OgBool
    sort_order: OgInt
    created_at: OgDateTime
    with_mail_templates: OgBool
    with_demo_data: OgBool
    required_server_type: OgStr

@dataclass
class RentedServer:
    """Rail class `RentedServer` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    provider: OgStr
    hostname: OgStr
    ip_address: OgStr
    woa_url: OgStr
    dns_eintrag: OgStr
    ssh_user: OgStr
    ssh_port: OgInt
    miete_netto: OgFloat
    miete_intervall: OgStr
    ssh_password_enc: OgStr
    root_password_enc: OgStr
    notizen_enc: OgStr
    master_api_url: OgStr
    master_api_token: OgStr
    verify_ssl: OgBool
    erp_ocr_token: OgStr
    last_sync_at: OgDateTime
    last_sync_status: OgStr
    last_sync_message: OgStr
    last_software_version: OgStr
    last_health_at: OgDateTime
    last_health_payload: OgStr
    sa_username: OgStr
    sa_password_enc: OgStr
    installed_at: OgDateTime
    tenant_slug_remote: OgStr
    is_master: OgBool
    aktiv: OgBool
    created_at: OgDateTime
    server_type: OgStr
    max_tenants: OgInt
    tenant_count: OgInt
    tenant_count_at: OgDateTime
    push_failure_count: OgInt
    push_last_failure_at: OgDateTime
    push_last_error: OgStr
    push_disabled: OgBool
    push_disabled_at: OgDateTime

@dataclass
class ServiceContract:
    """Rail class `ServiceContract` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    intervall: OgStr
    vertrag_start: OgDate
    rechnung_ab: OgDate
    naechste_rechnung: OgDate
    individualpreis_netto: OgFloat
    rabatt_prozent: OgFloat
    free_months_remaining: OgInt
    credit_eur: OgFloat
    bonus_pending_eur: OgFloat
    sales_partner_id: OgInt
    provision_pct: OgFloat
    commission_until: OgDate
    auto_versand: OgBool
    aktiv: OgBool
    gekuendigt_am: OgDate
    geloescht_am: OgDateTime
    notizen: OgStr
    created_at: OgDateTime
    last_invoice_at: OgDateTime
    last_invoice_workorder_id: OgInt
    shop_saas_config_id: OgInt
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
    firma: OgStr
    ansprechpartner: OgStr
    email: OgStr
    telefon: OgStr
    is_service_techniker: OgBool
    strasse: OgStr
    plz: OgStr
    ort: OgStr
    ustid: OgStr
    steuer_status: OgStr
    iban: OgStr
    bic: OgStr
    bank_name: OgStr
    commission_months: OgInt
    aktiv_ab: OgDate
    aktiv_bis: OgDate
    notizen: OgStr
    vertrag_pdf_path: OgStr
    vertrag_versendet_am: OgDateTime
    vertrag_versand_method: OgStr
    vertrag_signatur_token: OgStr
    vertrag_signiertes_pdf_path: OgStr
    vertrag_signiert_am: OgDateTime
    vertrag_signatur_data: OgStr
    vertrag_signatur_ip: OgStr
    vertrag_signatur_user_agent: OgStr
    created_at: OgDateTime
    updated_at: OgDateTime

@dataclass
class PartnerCommission:
    """Rail class `PartnerCommission` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    payout_id: OgInt
    tenant_id: OgInt
    basis_netto: OgFloat
    provision_pct: OgFloat
    betrag_netto: OgFloat
    status: OgStr
    pending_at: OgDateTime
    earned_at: OgDateTime
    paid_at: OgDate
    paid_workorder_id: OgInt
    cancelled_at: OgDateTime
    cancelled_reason: OgStr
    notes: OgStr
    created_at: OgDateTime
    partner: ToOne["SalesPartner"]
    contract: ToOne["ServiceContract"]
    source_workorder: ToOne["WorkOrder"]

@dataclass
class PartnerPayout:
    """Rail class `PartnerPayout` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    payout_nr: OgStr
    tenant_id: OgInt
    paid_at: OgDate
    summe_netto: OgFloat
    summe_ust: OgFloat
    summe_brutto: OgFloat
    ust_satz: OgFloat
    steuer_status_snapshot: OgStr
    note: OgStr
    pdf_path: OgStr
    pdf_generated_at: OgDateTime
    mail_sent_at: OgDateTime
    mail_sent_to: OgStr
    created_by: OgInt
    created_at: OgDateTime
    partner: ToOne["SalesPartner"]
    commissions: ToMany["PartnerCommission"]

@dataclass
class ContractSetup:
    """Rail class `ContractSetup` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    setup_phase: OgStr
    partner_decision: OgStr
    progress_pct: OgInt
    subdomain: OgStr
    server_ip: OgStr
    dns_recipient: OgStr
    dns_bcc: OgStr
    dns_mail_sent_at: OgDateTime
    dns_confirmed_at: OgDateTime
    dns_confirmed_by: OgInt
    server_ready_at: OgDateTime
    server_ready_by: OgInt
    server_notes: OgStr
    ssh_host: OgStr
    ssh_port: OgInt
    ssh_user: OgStr
    package_uploaded_at: OgDateTime
    package_path_remote: OgStr
    package_filename: OgStr
    target_tenant_id: OgInt
    target_tenant_name: OgStr
    target_tenant_slug: OgStr
    target_branche: OgStr
    welcome_recipient: OgStr
    welcome_sent_at: OgDateTime
    admin_username: OgStr
    onboarding_recipient: OgStr
    onboarding_bcc: OgStr
    onboarding_sent_at: OgDateTime
    onboarding_csv_attached: OgBool
    created_at: OgDateTime
    updated_at: OgDateTime
    contract: ToMany["ServiceContract"]

@dataclass
class ContractSetupHistory:
    """Rail class `ContractSetupHistory` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    contract_id: OgInt
    action: OgStr
    phase_from: OgStr
    phase_to: OgStr
    user_id: OgInt
    notes: OgStr
    created_at: OgDateTime

@dataclass
class AppVersion:
    """Rail class `AppVersion` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    version: OgStr
    notes: OgStr
    set_by: OgInt
    set_at: OgDateTime
    is_current: OgBool
    change_pdf_path: OgStr
    change_pdf_sha256: OgStr
    change_pdf_size: OgInt
    audit_detail_pdf_path: OgStr
    audit_detail_pdf_sha256: OgStr
    audit_detail_pdf_size: OgInt
    audit_layperson_pdf_path: OgStr
    audit_layperson_pdf_sha256: OgStr
    audit_layperson_pdf_size: OgInt
    audit_run_id: OgInt

@dataclass
class IpBlacklist:
    """Rail class `IpBlacklist` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    ip: OgStr
    grund: OgStr
    aktiv: OgBool
    auto: OgBool
    expires_at: OgDateTime
    created_at: OgDateTime
    created_by: OgInt
    last_hit_at: OgDateTime
    hit_count: OgInt
    safe_marked_by: OgInt
    safe_marked_at: OgDateTime
    safe_reason: OgStr
    origin_server: OgStr
    creator: ToOne["User"]
    safe_marker: ToOne["User"]

@dataclass
class LoginAudit:
    """Rail class `LoginAudit` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    username: OgStr
    ip: OgStr
    user_agent: OgStr
    success: OgBool
    reason: OgStr
    created_at: OgDateTime

@dataclass
class ScopeAuditBlock:
    """Rail class `ScopeAuditBlock` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    created_at: OgDateTime
    actor_user_id: OgInt
    actor_username: OgStr
    actor_tenant_id: OgInt
    actor_is_admin: OgBool
    actor_is_superadmin: OgBool
    target_model: OgStr
    target_id: OgInt
    target_tenant_id: OgInt
    route: OgStr
    method: OgStr
    reason: OgStr
    ip: OgStr
    user_agent: OgStr

@dataclass
class SecurityAudit:
    """Rail class `SecurityAudit` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    app_version: OgStr
    status: OgStr
    report_mode: OgStr
    audited_by: OgInt
    audited_by_name: OgStr
    audited_at: OgDateTime
    approved_by: OgInt
    approved_by_name: OgStr
    approved_at: OgDateTime
    results_json: OgStr
    overall_status: OgStr
    pass_count: OgInt
    fail_count: OgInt
    warn_count: OgInt
    skip_count: OgInt
    auditor_notes: OgStr
    version_history_json: OgStr
    signature_hash: OgStr
    signed_at: OgDateTime
    pdf_path: OgStr
    pdf_sha256: OgStr
    pdf_size: OgInt
    sa_notified_at: OgDateTime
    created_at: OgDateTime
    auditor: ToOne["User"]
    approver: ToOne["User"]

@dataclass
class SyncHistory:
    """Rail class `SyncHistory` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    rented_server_id: OgInt
    started_at: OgDateTime
    finished_at: OgDateTime
    action: OgStr
    status: OgStr
    files_changed: OgInt
    backup_path: OgStr
    error_message: OgStr
    log_excerpt: OgStr
    triggered_by: OgInt
    server: ToOne["RentedServer"]

@dataclass
class UpdateJob:
    """Rail class `UpdateJob` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    status: OgStr
    started_at: OgDateTime
    finished_at: OgDateTime
    last_heartbeat: OgDateTime
    current_step: OgStr
    progress_percent: OgInt
    log_excerpt: OgStr
    error_message: OgStr
    triggered_by: OgInt
    cancel_requested: OgBool
    server: ToOne["RentedServer"]

@dataclass
class UpdateSnapshot:
    """Rail class `UpdateSnapshot` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    snapshot_file: OgStr
    snapshot_size: OgInt
    version_before: OgStr
    version_after: OgStr
    status: OgStr
    log_excerpt: OgStr
    created_at: OgDateTime
    restored_at: OgDateTime
    rollback_note: OgStr

@dataclass
class TerminVorschlag:
    """Rail class `TerminVorschlag` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    project_id: OgInt
    defect_id: OgInt
    titel: OgStr
    beschreibung: OgStr
    termin_typ: OgStr
    vorschlaege_json: OgStr
    status: OgStr
    accepted_index: OgInt
    accepted_reminder_id: OgInt
    accepted_at: OgDateTime
    accepted_ip: OgStr
    token_slot1: OgStr
    token_slot2: OgStr
    token_slot3: OgStr
    token_decline: OgStr
    token_cancel: OgStr
    cancelled_at: OgDateTime
    cancelled_ip: OgStr
    expires_at: OgDateTime
    erstellt_am: OgDateTime
    updated_at: OgDateTime
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
    zeit_von: OgStr
    zeit_bis: OgStr
    caldav_uid: OgStr
    pushed_at: OgDateTime
    deleted_at: OgDateTime
    erstellt_am: OgDateTime
    tv: ToOne["TerminVorschlag"]

@dataclass
class HandbookFeature:
    """Rail class `HandbookFeature` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    titel: OgStr
    beschreibung: OgStr
    kategorie: OgStr
    version: OgStr
    reihenfolge: OgInt
    aktiv: OgBool
    datum: OgDate
    created_at: OgDateTime
    updated_at: OgDateTime

@dataclass
class BrancheTemplate:
    """Rail class `BrancheTemplate` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    branche: OgStr
    label: OgStr
    html_content: OgStr
    aktiv: OgBool
    erstellt_am: OgDateTime
    aktualisiert_am: OgDateTime

@dataclass
class ServiceContractItem:
    """Rail class `ServiceContractItem` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    sort_order: OgInt
    pos_typ: OgStr
    titel: OgStr
    beschreibung: OgStr
    intervall: OgStr
    preis_netto: OgFloat
    menge: OgFloat
    mwst_satz: OgFloat
    rabatt_typ: OgStr
    rabatt_wert: OgFloat
    rabatt_bis: OgDate
    rabatt_grund: OgStr
    aktiv_ab: OgDate
    aktiv_bis: OgDate
    abgerechnet_bis: OgDate
    sofort_rechnung: OgBool
    aktiv: OgBool
    created_at: OgDateTime
    contract: ToOne["ServiceContract"]
    package: ToOne["ServicePackage"]

@dataclass
class ContractBonus:
    """Rail class `ContractBonus` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    bonus_typ: OgStr
    wert: OgFloat
    grund: OgStr
    aktiv_ab: OgDate
    aktiv_bis: OgDate
    verbraucht: OgBool
    verbraucht_am: OgDateTime
    verbraucht_in_workorder_id: OgInt
    promo_code_id: OgInt
    aktiv: OgBool
    created_at: OgDateTime
    created_by: OgInt
    contract: ToMany["ServiceContract"]

@dataclass
class PromoCode:
    """Rail class `PromoCode` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    code: OgStr
    titel: OgStr
    beschreibung: OgStr
    code_typ: OgStr
    wert: OgFloat
    gueltig_ab: OgDate
    gueltig_bis: OgDate
    max_einloesungen: OgInt
    aktuelle_einloesungen: OgInt
    nur_neukunden: OgBool
    min_vertragswert_netto: OgFloat
    bonus_aktiv_monate: OgInt
    aktiv: OgBool
    created_at: OgDateTime
    created_by: OgInt

@dataclass
class OnboardingTemplate:
    """Rail class `OnboardingTemplate` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    kind: OgStr
    label: OgStr
    subject: OgStr
    html_content: OgStr
    plain_body: OgStr
    aktiv: OgBool
    erstellt_am: OgDateTime
    aktualisiert_am: OgDateTime

@dataclass
class PartnerContractTemplate:
    """Rail class `PartnerContractTemplate` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    label: OgStr
    html_content: OgStr
    aktiv: OgBool
    erstellt_am: OgDateTime
    aktualisiert_am: OgDateTime

@dataclass
class PortalAutoLoginToken:
    """Rail class `PortalAutoLoginToken` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    customer_portal_user_id: OgInt
    workorder_id: OgInt
    token: OgStr
    created_at: OgDateTime
    expires_at: OgDateTime
    last_used_at: OgDateTime
    revoked_at: OgDateTime
    use_count: OgInt
    created_by: OgStr
    scope: OgStr
    permanent: OgBool
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
    plain_body: OgStr
    updated_at: OgDateTime
    updated_by_user_id: OgInt

@dataclass
class TresorPwResetToken:
    """Rail class `TresorPwResetToken` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    customer_id: OgInt
    token: OgStr
    created_at: OgDateTime
    expires_at: OgDateTime
    used_at: OgDateTime
    created_by: OgStr

@dataclass
class LegacyRouteKey:
    """Rail class `LegacyRouteKey` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    key_data: OgStr
    retired_at: OgDateTime
    expires_at: OgDateTime
    note: OgStr

@dataclass
class RevokedToken:
    """Rail class `RevokedToken` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    token_hash: OgStr
    revoked_at: OgDateTime
    revoked_by_uid: OgInt
    reason: OgStr

@dataclass
class GeoblockSetting:
    """Rail class `GeoblockSetting` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    country_code: OgStr
    country_name: OgStr
    continent: OgStr
    blocked: OgBool
    updated_at: OgDateTime
    updated_by: OgInt

@dataclass
class GeoblockAllowIP:
    """Rail class `GeoblockAllowIP` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    cidr: OgStr
    note: OgStr
    created_at: OgDateTime
    created_by: OgInt

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
    tags: OgStr
    related_patches: OgStr
    is_active: OgBool
    created_at: OgDateTime
    created_by: OgStr

@dataclass
class ClaudeStaticSection:
    """Rail class `ClaudeStaticSection` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    position: OgInt
    section_key: OgStr
    title: OgStr
    content: OgStr
    has_timestamp_placeholder: OgBool
    is_active: OgBool
    created_at: OgDateTime
    updated_at: OgDateTime

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
    created_at: OgDateTime

@dataclass
class ShiftTicket:
    """Rail class `ShiftTicket` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    token: OgStr
    valid_until: OgDateTime
    created_at: OgDateTime
    created_by_id: OgInt
    used_at: OgDateTime
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
    tenant_id: OgInt
    rechtsform: OgStr
    festschreib_periode: OgStr
    kleinunternehmer: OgBool
    besteuerungsart: OgStr
    buchen_belegpflicht: OgBool
    lager_fibu_methode: OgStr
    lager_wareneinsatz_konto: OgStr
    bank_sachkonto_auto_pi: OgBool
    customer_bank_autosync: OgStr
    handelsregister: OgStr
    steuernummer: OgStr
    ust_id: OgStr
    default_currency: OgStr
    currency_custom_code: OgStr
    currency_custom_symbol: OgStr
    datev_beraternr: OgStr
    datev_mandantnr: OgStr
    datev_wj_beginn: OgInt
    erp_startdatum: OgDate
    legacy_source: OgStr
    legacy_object_id: OgStr
    created_at: OgDateTime
    updated_at: OgDateTime
    imap_server: OgStr
    imap_port: OgInt
    imap_user: OgStr
    imap_password: OgStr
    imap_folder: OgStr
    imap_ssl: OgBool
    imap_aktiv: OgBool
    imap_forward_to: OgStr
    imap_interval: OgInt
    gewerbesteuer_hebesatz: OgInt
    erechnung_format: OgStr

@dataclass
class ErpSupplierIban:
    """Rail class `ErpSupplierIban` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    supplier_id: OgInt
    iban: OgStr
    bic: OgStr
    notiz: OgStr
    created_at: OgDateTime

@dataclass
class ErpFintsInstitute:
    """Rail class `ErpFintsInstitute` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    blz: OgStr
    bic: OgStr
    institut: OgStr
    ort: OgStr
    rz: OgStr
    organisation: OgStr
    hbci_dns: OgStr
    hbci_version: OgStr
    pintan_url: OgStr
    fints_version: OgStr
    updated_at_src: OgDate
    imported_at: OgDateTime

@dataclass
class ErpExchangeRate:
    """Rail class `ErpExchangeRate` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    from_currency: OgStr
    to_currency: OgStr
    rate: OgMoney
    valid_from: OgDate
    valid_until: OgDate
    source: OgStr
    created_at: OgDateTime

@dataclass
class ErpEstTarifParams:
    """Rail class `ErpEstTarifParams` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    jahr: OgInt
    grundfreibetrag: OgInt
    zone2_bis: OgInt
    zone3_bis: OgInt
    zone4_bis: OgInt
    z2a: OgStr
    z2b: OgStr
    z3a: OgStr
    z3b: OgStr
    z3c: OgStr
    z4_abzug: OgStr
    z5_abzug: OgStr
    soli_freigrenze: OgInt
    created_at: OgDateTime
    updated_at: OgDateTime

@dataclass
class ErpKuGrenzen:
    """Rail class `ErpKuGrenzen` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    jahr: OgInt
    grenze_vorjahr: OgInt
    grenze_laufend: OgInt
    created_at: OgDateTime
    updated_at: OgDateTime

@dataclass
class ErpLedgerLock:
    """Rail class `ErpLedgerLock` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    buchungsperiode: OgStr
    status: OgStr
    locked_at: OgDateTime
    locked_by_user_id: OgInt
    hash_snapshot: OgStr
    created_at: OgDateTime

@dataclass
class ErpAuditTrail:
    """Rail class `ErpAuditTrail` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    seq_no: OgInt
    entity: OgStr
    entity_id: OgInt
    aktion: OgStr
    user_id: OgInt
    ip: OgStr
    user_agent: OgStr
    freitext: OgStr
    before_hash: OgStr
    after_hash: OgStr
    created_at: OgDateTime

@dataclass
class ErpChartOfAccounts:
    """Rail class `ErpChartOfAccounts` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    rahmen: OgStr
    aktiv: OgBool
    gesperrt_ab: OgDateTime
    created_at: OgDateTime

@dataclass
class ErpAccount:
    """Rail class `ErpAccount` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    rahmen: OgStr
    kontonummer: OgStr
    bezeichnung: OgStr
    kontoart: OgStr
    kontenklasse: OgInt
    steuer_relevant: OgBool
    ust_kennzeichen: OgStr
    automatik_konto: OgBool
    gesperrt: OgBool
    sort_order: OgInt
    currency: OgStr
    legacy_source: OgStr
    legacy_object_id: OgStr
    created_at: OgDateTime

@dataclass
class ErpCostCenter:
    """Rail class `ErpCostCenter` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    nummer: OgStr
    bezeichnung: OgStr
    aktiv: OgBool
    parent_id: OgInt
    created_at: OgDateTime

@dataclass
class ErpFiscalYear:
    """Rail class `ErpFiscalYear` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    jahr: OgInt
    beginn: OgDate
    ende: OgDate
    status: OgStr
    created_at: OgDateTime

@dataclass
class ErpPeriod:
    """Rail class `ErpPeriod` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    fiscal_year_id: OgInt
    periode_key: OgStr
    bezeichnung: OgStr
    status: OgStr
    created_at: OgDateTime

@dataclass
class ErpTaxAccountMap:
    """Rail class `ErpTaxAccountMap` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    rahmen: OgStr
    ust_kennzeichen: OgStr
    beschreibung: OgStr
    ust_konto: OgStr
    vst_konto: OgStr
    steuer_konto: OgStr
    prozent: OgMoney
    gueltig_ab: OgDate
    gueltig_bis: OgDate
    created_at: OgDateTime

@dataclass
class ErpJournalEntry:
    """Rail class `ErpJournalEntry` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    belegnummer: OgStr
    belegdatum: OgDate
    buchungsdatum: OgDate
    buchungstext: OgStr
    erfasst_von_user_id: OgInt
    festgeschrieben: OgBool
    festgeschrieben_am: OgDateTime
    storno_of_id: OgInt
    herkunft: OgStr
    herkunft_ref_id: OgInt
    currency: OgStr
    created_at: OgDateTime
    lines: ToMany["ErpJournalLine"]

@dataclass
class ErpJournalLine:
    """Rail class `ErpJournalLine` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    entry_id: OgInt
    konto: OgStr
    gegenkonto: OgStr
    soll_betrag: OgMoney
    haben_betrag: OgMoney
    steuer_betrag: OgMoney
    ust_kennzeichen: OgStr
    kostenstelle_id: OgInt
    zeilentext: OgStr

@dataclass
class ErpDebitorAccount:
    """Rail class `ErpDebitorAccount` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    customer_id: OgInt
    kontonummer: OgStr
    created_at: OgDateTime

@dataclass
class ErpSupplier:
    """Rail class `ErpSupplier` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    anschrift: OgStr
    ust_id: OgStr
    steuernummer: OgStr
    iban: OgStr
    bic: OgStr
    zahlungsziel: OgInt
    skonto_tage: OgInt
    skonto_prozent: OgFloat
    kreditorenkonto: OgStr
    aufwandskonto_default: OgStr
    ansprechpartner: OgStr
    ap_position: OgStr
    telefon: OgStr
    email: OgStr
    website: OgStr
    strasse: OgStr
    plz: OgStr
    ort: OgStr
    land: OgStr
    unsere_kundennr: OgStr
    notizen: OgStr
    lieferzeit_tage: OgInt
    mindestbestellwert: OgMoney
    aktiv: OgBool
    created_at: OgDateTime

@dataclass
class ErpPurchaseInvoice:
    """Rail class `ErpPurchaseInvoice` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    rechnungsnr: OgStr
    belegdatum: OgDate
    eingangsdatum: OgDate
    faellig_am: OgDate
    netto: OgMoney
    steuer: OgMoney
    brutto: OgMoney
    currency: OgStr
    aufwandskonto: OgStr
    buchungstext: OgStr
    split_buchung: OgStr
    geprueft: OgBool
    geprueft_von: OgInt
    geprueft_am: OgDateTime
    dokument_pfad: OgStr
    status: OgStr
    bezahlt: OgBool
    bezahlt_am: OgDate
    journal_entry_id: OgInt
    created_at: OgDateTime
    skontofrist: OgInt
    skontosatz: OgMoney
    supplier: ToOne["ErpSupplier"]

@dataclass
class ErpSupplierArticle:
    """Rail class `ErpSupplierArticle` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    lieferanten_artikelnr: OgStr
    ek_preis: OgMoney
    currency: OgStr
    mindestmenge: OgInt
    staffelpreise: OgStr
    lieferzeit_tage: OgInt
    ist_hauptlieferant: OgBool
    aktiv: OgBool
    created_at: OgDateTime
    supplier: ToOne["ErpSupplier"]
    article: ToOne["Article"]

@dataclass
class ErpPurchaseOrder:
    """Rail class `ErpPurchaseOrder` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    bestellnummer: OgStr
    status: OgStr
    bestelldatum: OgDate
    liefertermin: OgDate
    netto_summe: OgMoney
    steuer_summe: OgMoney
    brutto_summe: OgMoney
    currency: OgStr
    notizen: OgStr
    purchase_invoice_id: OgInt
    erstellt_von: OgInt
    created_at: OgDateTime
    updated_at: OgDateTime
    supplier: ToOne["ErpSupplier"]
    lines: ToMany["ErpPurchaseOrderLine"]

@dataclass
class ErpPurchaseOrderLine:
    """Rail class `ErpPurchaseOrderLine` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    purchase_order_id: OgInt
    artikelnr: OgStr
    bezeichnung: OgStr
    menge: OgFloat
    ek_preis: OgMoney
    steuersatz: OgMoney
    currency: OgStr
    geliefert_menge: OgFloat
    notiz: OgStr
    article: ToOne["Article"]

@dataclass
class ErpMaterialBedarf:
    """Rail class `ErpMaterialBedarf` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    menge_position: OgFloat
    menge_bestand: OgFloat
    menge_bedarf: OgFloat
    menge_bestellt: OgFloat
    menge_geliefert: OgFloat
    einheit: OgStr
    best_ek: OgMoney
    status: OgStr
    workorder_nr: OgStr
    artikel_bez: OgStr
    created_at: OgDateTime
    created_by: OgInt
    notiz: OgStr
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
    tenant_id: OgInt
    name: OgStr
    sachkonto: OgStr
    currency: OgStr
    aktiv: OgBool
    anfangssaldo: OgMoney
    created_at: OgDateTime

@dataclass
class ErpCashEntry:
    """Rail class `ErpCashEntry` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    lfd_nr: OgInt
    datum: OgDate
    vorgang: OgStr
    betrag: OgMoney
    gegenkonto: OgStr
    zweck: OgStr
    erfasst_von_user_id: OgInt
    festgeschrieben: OgBool
    storno_of_id: OgInt
    journal_entry_id: OgInt
    created_at: OgDateTime
    register: ToOne["ErpCashRegister"]

@dataclass
class ErpDocument:
    """Rail class `ErpDocument` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    original_name: OgStr
    stored_name: OgStr
    mime_type: OgStr
    size_bytes: OgInt
    integrity_hash: OgStr
    belegtyp: OgStr
    beleg_status: OgStr
    steuerberater_relevant: OgBool
    beschreibung: OgStr
    customer_id: OgInt
    purchase_invoice_id: OgInt
    asset_id: OgInt
    journal_entry_id: OgInt
    legacy_source: OgStr
    legacy_object_id: OgStr
    legacy_tree_id: OgStr
    legacy_filename: OgStr
    legacy_imported_at: OgDateTime
    uploaded_by: OgInt
    uploaded_at: OgDateTime
    kategorie_id: OgInt
    periode: OgStr
    tags: OgStr
    beleg_datum: OgDate
    lieferant_id: OgInt
    workorder_id: OgInt
    bank_tx_id: OgInt
    ocr_status: OgStr

@dataclass
class ErpDocumentFulltext:
    """Rail class `ErpDocumentFulltext` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    quelle: OgStr
    text: OgStr
    indexed_at: OgDateTime
    document: ToOne["ErpDocument"]

@dataclass
class ErpDmsKategorie:
    """Rail class `ErpDmsKategorie` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    parent_id: OgInt
    name: OgStr
    beschreibung: OgStr
    icon: OgStr
    sort_order: OgInt
    aktiv: OgBool
    created_at: OgDateTime
    children: ToMany["ErpDmsKategorie"]

@dataclass
class ErpDmsAudit:
    """Rail class `ErpDmsAudit` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    document_id: OgInt
    user_id: OgInt
    aktion: OgStr
    von_status: OgStr
    zu_status: OgStr
    kommentar: OgStr
    created_at: OgDateTime

@dataclass
class SambaShare:
    """Rail class `SambaShare` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    path: OgStr
    comment: OgStr
    browseable: OgBool
    aktiv: OgBool
    created_at: OgDateTime

@dataclass
class SambaShareAcl:
    """Rail class `SambaShareAcl` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    can_read: OgBool
    can_write: OgBool
    share: ToOne["SambaShare"]
    user: ToOne["User"]

@dataclass
class ScanRequest:
    """Rail class `ScanRequest` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    user_id: OgInt
    ziel_typ: OgStr
    ziel_id: OgInt
    status: OgStr
    document_id: OgInt
    created_at: OgDateTime
    completed_at: OgDateTime
    expires_at: OgDateTime
    hinweis: OgStr

@dataclass
class EinsatzSession:
    """Rail class `EinsatzSession` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    pairing_code: OgStr
    qr_token: OgStr
    code_expires_at: OgDateTime
    session_token: OgStr
    user_id: OgInt
    customer_id: OgInt
    reminder_id: OgInt
    workorder_id: OgInt
    scope: OgStr
    status: OgStr
    paired_at: OgDateTime
    expires_at: OgDateTime
    last_activity: OgDateTime
    ended_at: OgDateTime
    ended_by: OgStr
    browser_fp: OgStr
    ip_address: OgStr
    user_agent: OgStr
    created_at: OgDateTime

@dataclass
class ErpSerialCharge:
    """Rail class `ErpSerialCharge` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    typ: OgStr
    nummer: OgStr
    mhd: OgDate
    status: OgStr
    workorder_id: OgInt
    movement_id: OgInt
    created_at: OgDateTime
    article: ToOne["Article"]

@dataclass
class ErpEinvoiceImport:
    """Rail class `ErpEinvoiceImport` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    status: OgStr
    filename: OgStr
    dateityp: OgStr
    raw_xml: OgStr
    profil: OgStr
    format_typ: OgStr
    lieferant_name: OgStr
    lieferant_ustid: OgStr
    lieferant_iban: OgStr
    rechnungsnr: OgStr
    rechnungsdatum: OgDate
    faelligkeit: OgDate
    waehrung: OgStr
    netto_betrag: OgMoney
    steuer_betrag: OgMoney
    brutto_betrag: OgMoney
    steuer_prozent: OgMoney
    verwendungszweck: OgStr
    positionen_json: OgStr
    purchase_invoice_id: OgInt
    document_id: OgInt
    fehler: OgStr
    warnungen: OgStr
    hash_dedup: OgStr
    created_at: OgDateTime
    created_by: OgInt
    confirmed_at: OgDateTime
    confirmed_by: OgInt
    supplier: ToOne["ErpSupplier"]

@dataclass
class ErpSupplierCsvMapping:
    """Rail class `ErpSupplierCsvMapping` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    trennzeichen: OgStr
    encoding: OgStr
    header_zeile: OgInt
    skip_zeilen: OgInt
    col_ean: OgStr
    col_artikelnr: OgStr
    col_bezeichnung: OgStr
    col_ek_preis: OgStr
    col_mindestmenge: OgStr
    col_einheit: OgStr
    col_hersteller: OgStr
    col_hersteller_anr: OgStr
    col_bild_url: OgStr
    col_kategorie: OgStr
    col_unterkategorie: OgStr
    fuzzy_schwelle: OgInt
    auto_anlegen: OgBool
    auto_kategorie: OgStr
    auto_update_url: OgStr
    auto_update_aktiv: OgBool
    auto_update_auth_type: OgStr
    auto_update_auth_user: OgStr
    auto_update_auth_pass: OgStr
    auto_update_auth_header: OgStr
    auto_update_omd_token_url: OgStr
    auto_update_omd_client_id: OgStr
    auto_update_omd_client_secret: OgStr
    auto_update_omd_cred_location: OgStr
    auto_update_omd_customer_id: OgStr
    auto_update_intervall: OgInt
    auto_update_typ: OgStr
    auto_update_uhrzeit: OgStr
    auto_update_wochentage: OgStr
    auto_update_next_run: OgDateTime
    auto_inaktiv_setzen: OgBool
    auto_update_last_status: OgStr
    auto_update_last_error: OgStr
    auto_update_last_run: OgDateTime
    last_import_at: OgDateTime
    last_import_count: OgInt
    created_at: OgDateTime
    supplier: ToOne["ErpSupplier"]

@dataclass
class ErpStockMovement:
    """Rail class `ErpStockMovement` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    datum: OgDate
    bewegungsart: OgStr
    menge: OgFloat
    ek_preis: OgFloat
    currency: OgStr
    herkunft: OgStr
    herkunft_ref_id: OgInt
    notiz: OgStr
    erfasst_von: OgInt
    erfasst_am: OgDateTime
    mitarbeiter_id: OgInt
    supplier_id: OgInt
    gutschrift_nr: OgStr
    schadensgrund: OgStr
    article: ToOne["Article"]

@dataclass
class ErpAsset:
    """Rail class `ErpAsset` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    inventarnr: OgStr
    bezeichnung: OgStr
    anlagenklasse: OgStr
    anschaffungsdatum: OgDate
    anschaffungskosten: OgMoney
    currency: OgStr
    nutzungsdauer_jahre: OgInt
    afa_methode: OgStr
    anlagenkonto: OgStr
    afa_konto: OgStr
    kum_afa: OgMoney
    letztes_afa_jahr: OgInt
    aktiv: OgBool
    abgang_datum: OgDate
    abgang_art: OgStr
    device_id: OgInt
    kostenstelle_id: OgInt
    created_at: OgDateTime

@dataclass
class ErpBankAccount:
    """Rail class `ErpBankAccount` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    iban: OgStr
    bic: OgStr
    sachkonto: OgStr
    aktiv: OgBool
    kontotyp: OgStr
    ist_hauptkonto: OgBool
    anfangssaldo: OgMoney
    fints_aktiv: OgBool
    fints_blz: OgStr
    fints_endpoint_url: OgStr
    fints_login: OgStr
    fints_pin_encrypted: OgStr
    fints_tan_methode: OgStr
    fints_letzter_abruf: OgDateTime
    fints_status: OgStr
    fints_letzter_fehler: OgStr
    fints_system_id: OgStr
    created_at: OgDateTime

@dataclass
class ErpBankTransaction:
    """Rail class `ErpBankTransaction` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    bank_account_id: OgInt
    buchungsdatum: OgDate
    betrag: OgMoney
    verwendungszweck: OgStr
    gegenname: OgStr
    gegen_iban: OgStr
    status: OgStr
    match_wo_id: OgInt
    match_pi_id: OgInt
    match_type: OgStr
    match_transfer_tx_id: OgInt
    match_sachkonto: OgStr
    match_steuersatz: OgInt
    import_hash: OgStr
    created_at: OgDateTime
    document_id: OgInt
    match_darlehen_id: OgInt
    match_split_data: OgStr
    match_abschlag_id: OgInt
    match_konto_override: OgStr
    hinweis_text: OgStr

@dataclass
class ErpDarlehen:
    """Rail class `ErpDarlehen` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    darlehensnr: OgStr
    bezeichnung: OgStr
    glaeubiger: OgStr
    glaeubiger_iban: OgStr
    darlehensbetrag_brutto: OgMoney
    restschuld: OgMoney
    zinssatz: OgMoney
    startdatum: OgDate
    enddatum_geplant: OgDate
    monatsrate_soll: OgMoney
    konto_darlehen: OgStr
    konto_zinsen: OgStr
    aktiv: OgBool
    notiz: OgStr
    created_at: OgDateTime

@dataclass
class ErpAbschlagskonto:
    """Rail class `ErpAbschlagskonto` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    bezeichnung: OgStr
    anbieter: OgStr
    anbieter_iban: OgStr
    vertragsnummer: OgStr
    art: OgStr
    monatlicher_abschlag: OgMoney
    konto_aufwand: OgStr
    ust_satz: OgInt
    konto_aufwand_2: OgStr
    anteil_aufwand_2_proz: OgInt
    aktiv: OgBool
    notiz: OgStr
    created_at: OgDateTime

@dataclass
class ErpPaymentRun:
    """Rail class `ErpPaymentRun` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    ausfuehrungsdatum: OgDate
    status: OgStr
    summe: OgMoney
    anzahl: OgInt
    currency: OgStr
    xml_exported: OgBool
    created_at: OgDateTime
    created_by_id: OgInt
    bank_account: ToOne["ErpBankAccount"]
    items: ToMany["ErpPaymentRunItem"]

@dataclass
class ErpPaymentRunItem:
    """Rail class `ErpPaymentRunItem` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    run_id: OgInt
    purchase_invoice_id: OgInt
    empfaenger: OgStr
    iban: OgStr
    bic: OgStr
    betrag: OgMoney
    currency: OgStr
    verwendungszweck: OgStr

@dataclass
class ErpVatReturn:
    """Rail class `ErpVatReturn` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    jahr: OgInt
    monat: OgInt
    kz81: OgMoney
    kz86: OgMoney
    kz66: OgMoney
    ust_19: OgMoney
    ust_7: OgMoney
    zahllast: OgMoney
    status: OgStr
    berechnet_am: OgDateTime
    created_at: OgDateTime

@dataclass
class ErpPreisSchema:
    """Rail class `ErpPreisSchema` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    beschreibung: OgStr
    fix_aufschlag: OgMoney
    rundung_typ: OgStr
    mindestmarge_pct: OgMoney
    auto_neuberechnung: OgBool
    ist_default: OgBool
    aktiv: OgBool
    created_at: OgDateTime
    updated_at: OgDateTime
    spannen: ToMany["ErpPreisSpanne"]

@dataclass
class ErpPreisSpanne:
    """Rail class `ErpPreisSpanne` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    schema_id: OgInt
    preis_von: OgMoney
    preis_bis: OgMoney
    aufschlag_pct: OgMoney
    sort_order: OgInt

@dataclass
class ErpPreisGruppe:
    """Rail class `ErpPreisGruppe` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    rabatt_pct: OgMoney
    schema_id: OgInt
    ist_default: OgBool
    sort_order: OgInt
    aktiv: OgBool
    created_at: OgDateTime
    kunden: ToMany["Customer"]

@dataclass
class ErpLohnPeriode:
    """Rail class `ErpLohnPeriode` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    jahr: OgInt
    monat: OgInt
    brutto_loehne: OgMoney
    ag_sv_anteile: OgMoney
    an_sv_anteile: OgMoney
    lohnsteuer_gesamt: OgMoney
    currency: OgStr
    notiz: OgStr
    gebucht_am: OgDateTime
    gebucht_von: OgInt
    erfasst_am: OgDateTime
    erfasst_von: OgInt

@dataclass
class DashboardDismissal:
    """Rail class `DashboardDismissal` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    user_id: OgInt
    item_key: OgStr
    created_at: OgDateTime

@dataclass
class CustomerRemoteDevice:
    """Rail class `CustomerRemoteDevice` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    provider: OgStr
    label: OgStr
    device_id: OgStr
    notizen: OgStr
    created_at: OgDateTime
    customer: ToOne["Customer"]

@dataclass
class CrmCompany:
    """Rail class `CrmCompany` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    rechtsform: OgStr
    branche: OgStr
    website: OgStr
    telefon: OgStr
    email: OgStr
    strasse: OgStr
    plz: OgStr
    ort: OgStr
    land: OgStr
    ust_id: OgStr
    customer_id: OgInt
    inhaber_user_id: OgInt
    notizen: OgStr
    tags: OgStr
    aktiv: OgBool
    geloescht_am: OgDateTime
    erstellt_am: OgDateTime
    geaendert_am: OgDateTime

@dataclass
class CrmContact:
    """Rail class `CrmContact` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    company_id: OgInt
    customer_id: OgInt
    anrede: OgStr
    vorname: OgStr
    nachname: OgStr
    position: OgStr
    abteilung: OgStr
    email: OgStr
    telefon: OgStr
    mobil: OgStr
    ist_hauptkontakt: OgBool
    quelle: OgStr
    notizen: OgStr
    tags: OgStr
    aktiv: OgBool
    geloescht_am: OgDateTime
    erstellt_am: OgDateTime
    geaendert_am: OgDateTime
    carddav_uid: OgStr
    carddav_href: OgStr
    carddav_etag: OgStr
    carddav_synced_at: OgDateTime

@dataclass
class CrmPipeline:
    """Rail class `CrmPipeline` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    ist_default: OgBool
    sort_order: OgInt
    aktiv: OgBool
    geloescht_am: OgDateTime
    erstellt_am: OgDateTime

@dataclass
class CrmStage:
    """Rail class `CrmStage` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    pipeline_id: OgInt
    name: OgStr
    sort_order: OgInt
    wahrscheinlichkeit: OgInt
    ist_gewonnen: OgBool
    ist_verloren: OgBool
    farbe: OgStr
    aktiv: OgBool
    geloescht_am: OgDateTime

@dataclass
class CrmLead:
    """Rail class `CrmLead` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    titel: OgStr
    pipeline_id: OgInt
    stage_id: OgInt
    contact_id: OgInt
    company_id: OgInt
    customer_id: OgInt
    wert_netto: OgMoney
    waehrung: OgStr
    wahrscheinlichkeit: OgInt
    quelle: OgStr
    zustaendig_user_id: OgInt
    erwartetes_datum: OgDate
    status: OgStr
    verloren_grund: OgStr
    source_cold_lead_id: OgInt
    notizen: OgStr
    tags: OgStr
    category_id: OgInt
    aktiv: OgBool
    geloescht_am: OgDateTime
    erstellt_am: OgDateTime
    geaendert_am: OgDateTime

@dataclass
class CrmCategory:
    """Rail class `CrmCategory` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    farbe: OgStr
    sort_order: OgInt
    aktiv: OgBool
    geloescht_am: OgDateTime
    erstellt_am: OgDateTime

@dataclass
class CrmActivity:
    """Rail class `CrmActivity` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    entity_type: OgStr
    entity_id: OgInt
    typ: OgStr
    betreff: OgStr
    inhalt: OgStr
    richtung: OgStr
    ref_table: OgStr
    ref_id: OgInt
    user_id: OgInt
    zeitpunkt: OgDateTime
    geloescht_am: OgDateTime
    erstellt_am: OgDateTime

@dataclass
class CrmTask:
    """Rail class `CrmTask` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    titel: OgStr
    beschreibung: OgStr
    entity_type: OgStr
    entity_id: OgInt
    faellig_am: OgDateTime
    erinnerung_am: OgDateTime
    prioritaet: OgStr
    status: OgStr
    zustaendig_user_id: OgInt
    erledigt_am: OgDateTime
    geloescht_am: OgDateTime
    erstellt_am: OgDateTime
    geaendert_am: OgDateTime

@dataclass
class CrmDocument:
    """Rail class `CrmDocument` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    titel: OgStr
    vorlage_id: OgInt
    entity_type: OgStr
    entity_id: OgInt
    customer_id: OgInt
    storage_key: OgStr
    mime_type: OgStr
    format: OgStr
    version: OgInt
    erp_document_id: OgInt
    status: OgStr
    erstellt_von: OgInt
    folder_id: OgInt
    aktiv: OgBool
    geloescht_am: OgDateTime
    erstellt_am: OgDateTime
    geaendert_am: OgDateTime

@dataclass
class CrmDocumentFolder:
    """Rail class `CrmDocumentFolder` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    entity_type: OgStr
    entity_id: OgInt
    parent_id: OgInt
    name: OgStr
    sort_order: OgInt
    geloescht_am: OgDateTime
    erstellt_am: OgDateTime

@dataclass
class CrmDocTemplate:
    """Rail class `CrmDocTemplate` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    name: OgStr
    format: OgStr
    storage_key: OgStr
    beschreibung: OgStr
    kategorie: OgStr
    aktiv: OgBool
    geloescht_am: OgDateTime
    erstellt_am: OgDateTime

@dataclass
class CrmDocumentLink:
    """Rail class `CrmDocumentLink` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    tenant_id: OgInt
    erp_document_id: OgInt
    entity_type: OgStr
    entity_id: OgInt
    zugeordnet_von_user_id: OgInt
    zugeordnet_am: OgDateTime
    quelle: OgStr
    geloescht_am: OgDateTime
    erstellt_am: OgDateTime

@dataclass
class RentedServerEvent:
    """Rail class `RentedServerEvent` — classid 0x00000000 (concept 0x0000, app 0x0000)."""
    CLASSID: ClassVar[int] = 0x00000000
    id: OgInt
    ts: OgDateTime
    kind: OgStr
    level: OgStr
    message: OgStr
    payload: OgStr
    server: ToOne["RentedServer"]

