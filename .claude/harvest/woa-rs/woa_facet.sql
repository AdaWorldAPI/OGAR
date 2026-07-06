-- WoA sink-in substrate v1 — PostgreSQL DDL, generated, do not hand-edit.
-- facet table: classid(4) + 12-byte payload (V3 sink-in System-of-Record)

CREATE TABLE facet (
    classid INTEGER NOT NULL,
    p0 SMALLINT,
    p1 SMALLINT,
    p2 SMALLINT,
    p3 SMALLINT,
    p4 SMALLINT,
    p5 SMALLINT,
    p6 SMALLINT,
    p7 SMALLINT,
    p8 SMALLINT,
    p9 SMALLINT,
    p10 SMALLINT,
    p11 SMALLINT
);
CREATE INDEX facet_classid_idx ON facet (classid);
CREATE INDEX facet_p0_idx ON facet (p0);
CREATE INDEX facet_p1_idx ON facet (p1);
CREATE INDEX facet_p2_idx ON facet (p2);
CREATE INDEX facet_p3_idx ON facet (p3);
CREATE INDEX facet_p4_idx ON facet (p4);
CREATE INDEX facet_p5_idx ON facet (p5);
CREATE INDEX facet_p6_idx ON facet (p6);
CREATE INDEX facet_p7_idx ON facet (p7);
CREATE INDEX facet_p8_idx ON facet (p8);
CREATE INDEX facet_p9_idx ON facet (p9);
CREATE INDEX facet_p10_idx ON facet (p10);
CREATE INDEX facet_p11_idx ON facet (p11);

-- per-class relational DDL (ClassView-projected, typed + nullable)

CREATE TABLE Tenant (
    id INTEGER NOT NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    max_users INTEGER,
    aktiv BOOLEAN,
    logo_path TEXT,
    logo_mail_path TEXT,
    created_at TIMESTAMP,
    branche TEXT,
    is_test BOOLEAN,
    is_anbieter BOOLEAN,
    mod_tresor BOOLEAN,
    mod_stundenzettel BOOLEAN,
    mod_multitimer BOOLEAN,
    mod_fahrtenbuch BOOLEAN,
    mod_wiedervorlage BOOLEAN,
    mod_notizbuch BOOLEAN,
    mod_wartung BOOLEAN,
    mod_abo BOOLEAN,
    mod_inventar BOOLEAN,
    mod_abnahme BOOLEAN,
    mod_gobd BOOLEAN,
    mod_referral BOOLEAN,
    mod_kaltakquise BOOLEAN,
    mod_woa_service BOOLEAN,
    mod_erp BOOLEAN,
    mod_dms BOOLEAN,
    mod_rustdesk BOOLEAN,
    mod_rustdesk_server BOOLEAN,
    erp_gobd_festschreibung BOOLEAN,
    erp_mod_stammdaten BOOLEAN,
    erp_mod_fibu BOOLEAN,
    erp_mod_bank BOOLEAN,
    erp_mod_steuer BOOLEAN,
    erp_mod_lager BOOLEAN,
    erp_mod_dms BOOLEAN,
    erp_mod_reporting BOOLEAN,
    erp_mod_pos BOOLEAN,
    erp_mod_zugferd BOOLEAN,
    erp_mod_lohn BOOLEAN,
    erp_mod_shop BOOLEAN,
    erp_mod_crm BOOLEAN,
    erp_mod_stammdaten_gesperrt BOOLEAN,
    erp_mod_fibu_gesperrt BOOLEAN,
    erp_mod_bank_gesperrt BOOLEAN,
    erp_mod_steuer_gesperrt BOOLEAN,
    erp_mod_lager_gesperrt BOOLEAN,
    erp_mod_dms_gesperrt BOOLEAN,
    erp_mod_reporting_gesperrt BOOLEAN,
    erp_mod_pos_gesperrt BOOLEAN,
    erp_mod_zugferd_gesperrt BOOLEAN,
    erp_mod_crm_gesperrt BOOLEAN,
    erp_mod_lohn_gesperrt BOOLEAN,
    erp_mod_shop_gesperrt BOOLEAN
);

CREATE TABLE User (
    id INTEGER NOT NULL,
    username TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    firstname TEXT,
    lastname TEXT,
    email TEXT,
    phone TEXT,
    ma_rabatt DOUBLE PRECISION,
    is_admin BOOLEAN,
    is_superadmin BOOLEAN,
    perm_dashboard_umsaetze BOOLEAN NOT NULL,
    perm_buchhaltung BOOLEAN NOT NULL,
    perm_statistik BOOLEAN NOT NULL,
    perm_einstellungen BOOLEAN NOT NULL,
    perm_dms BOOLEAN NOT NULL,
    perm_serverschutz BOOLEAN NOT NULL,
    perm_erp_stammdaten BOOLEAN NOT NULL,
    perm_erp_buchen BOOLEAN NOT NULL,
    perm_erp_debitoren BOOLEAN NOT NULL,
    perm_erp_kreditoren BOOLEAN NOT NULL,
    perm_erp_bank BOOLEAN NOT NULL,
    perm_erp_kasse BOOLEAN NOT NULL,
    perm_erp_steuer BOOLEAN NOT NULL,
    perm_erp_abschluss BOOLEAN NOT NULL,
    perm_erp_anlagen BOOLEAN NOT NULL,
    perm_erp_lager BOOLEAN NOT NULL,
    perm_erp_einkauf BOOLEAN NOT NULL,
    perm_erp_dms BOOLEAN NOT NULL,
    perm_erp_compliance BOOLEAN NOT NULL,
    perm_erp_lohn BOOLEAN NOT NULL,
    failed_attempts INTEGER,
    locked_until TIMESTAMP,
    scan_import_mode TEXT,
    scan_import_username TEXT,
    scan_import_pw_set_at TIMESTAMP,
    scan_import_host TEXT,
    scan_import_port INTEGER,
    vpn_enabled BOOLEAN NOT NULL,
    vpn_ip TEXT,
    vpn_pubkey TEXT,
    vpn_created_at TIMESTAMP,
    samba_enabled BOOLEAN NOT NULL,
    samba_pw_set_at TIMESTAMP,
    created_at TIMESTAMP
);

CREATE TABLE Customer (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    kdnr TEXT,
    quick_token TEXT,
    quick_token_ts TEXT,
    firma TEXT,
    anrede TEXT,
    vorname TEXT,
    nachname TEXT,
    mail_anrede TEXT,
    strasse TEXT,
    adresszusatz TEXT,
    plz TEXT,
    ort TEXT,
    email TEXT,
    telefon TEXT,
    tresor_pw_hash TEXT,
    tresor_pw_set_at TIMESTAMP,
    tresor_pw_failed INTEGER,
    tresor_pw_locked_until TIMESTAMP,
    zahlungsziel INTEGER,
    skonto_prozent DOUBLE PRECISION,
    skonto_tage INTEGER,
    stundensatz DOUBLE PRECISION,
    fahrt_km DOUBLE PRECISION,
    fahrt_kosten DOUBLE PRECISION,
    notizen TEXT,
    aktiv BOOLEAN,
    kundentyp TEXT,
    referral_code TEXT,
    sepa_iban TEXT,
    sepa_bic TEXT,
    sepa_kontoinhaber TEXT,
    sepa_mandat_ref TEXT,
    sepa_mandat_datum DATE,
    sepa_mandat_typ TEXT,
    sepa_mandat_status TEXT,
    sepa_letzte_lastschrift DATE,
    sepa_pre_notification_tage INTEGER,
    preis_gruppe_id INTEGER,
    created_at TIMESTAMP
);

CREATE TABLE Project (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    name TEXT NOT NULL,
    projekt_nr TEXT,
    beschreibung TEXT,
    status TEXT,
    sort_order INTEGER,
    erstellt_am TIMESTAMP,
    erstellt_von_id INTEGER,
    abgeschlossen_am TIMESTAMP
);

CREATE TABLE ProjectNote (
    id INTEGER NOT NULL,
    titel TEXT,
    inhalt_text TEXT,
    inhalt_zeichnung TEXT,
    erstellt_am TIMESTAMP,
    erstellt_von_id INTEGER
);

CREATE TABLE ErpArticleCategory (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    name TEXT NOT NULL,
    icon TEXT,
    sort_order INTEGER,
    created_at TIMESTAMP
);

CREATE TABLE ErpStorageLocation (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    name TEXT NOT NULL,
    kuerzel TEXT,
    icon TEXT,
    beschreibung TEXT,
    sort_order INTEGER,
    aktiv BOOLEAN,
    created_at TIMESTAMP
);

CREATE TABLE Article (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    artikelnr TEXT,
    ean TEXT,
    beschreibung TEXT NOT NULL,
    kategorie TEXT,
    category_id INTEGER,
    storage_location_id INTEGER,
    einheit TEXT,
    hersteller TEXT,
    hersteller_anr TEXT,
    bild_url TEXT,
    bestand DOUBLE PRECISION,
    mindestbestand DOUBLE PRECISION,
    preis_netto NUMERIC,
    ek_preis NUMERIC,
    mwst_satz DOUBLE PRECISION,
    tax_rate_id INTEGER,
    lieferant TEXT,
    lieferant_anr TEXT,
    notizen TEXT,
    typ TEXT,
    aktiv BOOLEAN,
    dauer_minuten INTEGER,
    preis_schema_id INTEGER,
    vk_preis_manuell BOOLEAN,
    listenpreis NUMERIC,
    uvp NUMERIC,
    gewicht_kg NUMERIC,
    herkunftsland TEXT,
    zolltarifnr TEXT,
    langbeschreibung TEXT,
    matchcode TEXT,
    warengruppe TEXT,
    warengruppe_nr TEXT,
    gefahrgut BOOLEAN,
    gefahrgut_un_nr TEXT,
    gefahrgut_klasse TEXT,
    auslaufartikel BOOLEAN,
    auslaufdatum DATE,
    deeplink TEXT,
    shop_active BOOLEAN,
    shop_product_type TEXT,
    shop_category_id INTEGER,
    shop_long_desc_html TEXT,
    shop_saas_meta_json TEXT,
    shop_saas_package_id INTEGER,
    shop_digital_path TEXT,
    shop_meta_title TEXT,
    shop_meta_description TEXT,
    shop_slug TEXT,
    shop_payment_methods_csv TEXT,
    shop_free_shipping BOOLEAN NOT NULL,
    shop_extra_user_price_cents INTEGER,
    shop_addon_article_ids TEXT,
    shop_included_users INTEGER NOT NULL,
    omd_tax_code INTEGER
);

CREATE TABLE WorkOrder (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    customer_id INTEGER NOT NULL,
    created_by INTEGER,
    doc_type TEXT,
    status TEXT,
    angebot_nr TEXT,
    auftrags_nr TEXT,
    workorder_nr TEXT,
    rechnung_nr TEXT,
    gutschrift_nr TEXT,
    sammelrechnung_id INTEGER,
    datum DATE,
    zeit_start TEXT,
    zeit_ende TEXT,
    anfahrten DOUBLE PRECISION,
    mitarbeiter DOUBLE PRECISION,
    pause_h DOUBLE PRECISION,
    zusatz_h DOUBLE PRECISION,
    betreff TEXT,
    notizen TEXT,
    intern_notizen TEXT,
    bezahlt BOOLEAN,
    bezahlt_am DATE,
    bezahlt_betrag NUMERIC,
    mahnstufe INTEGER,
    letzte_mahnung DATE,
    erfuellung_bis DATE,
    unterschrift TEXT,
    signed_at TIMESTAMP,
    signed_ip TEXT,
    signed_user_agent TEXT,
    zahlungsart TEXT,
    anzahlung_prozent DOUBLE PRECISION,
    anzahlung_betrag DOUBLE PRECISION,
    anzahlung_bezahlt BOOLEAN,
    anzahlung_bezahlt_am DATE,
    kleinunternehmer_snapshot BOOLEAN,
    zahlungsziel_tage_snapshot INTEGER,
    gesamt_rabatt_prozent DOUBLE PRECISION,
    gesamt_rabatt_betrag DOUBLE PRECISION,
    skonto_prozent_snapshot DOUBLE PRECISION,
    skonto_tage_snapshot INTEGER,
    skonto_ausweisen BOOLEAN,
    skonto_aufschlag BOOLEAN,
    skonto_aufschlag_faktor DOUBLE PRECISION,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE TABLE Position (
    id INTEGER NOT NULL,
    workorder_id INTEGER NOT NULL,
    article_id INTEGER,
    sort_order INTEGER,
    pos_typ TEXT,
    beschreibung TEXT,
    menge DOUBLE PRECISION,
    einheit TEXT,
    einzelpreis NUMERIC,
    mwst_satz DOUBLE PRECISION,
    tax_rate_id INTEGER,
    versteckt BOOLEAN,
    is_optional BOOLEAN,
    customer_accepted_at TIMESTAMP,
    rabatt_prozent DOUBLE PRECISION,
    einzelpreis_vor_skonto NUMERIC
);

CREATE TABLE Activity (
    id INTEGER NOT NULL,
    workorder_id INTEGER NOT NULL,
    geraet TEXT,
    beschreibung TEXT,
    logbuch BOOLEAN,
    intern BOOLEAN,
    created_at TIMESTAMP
);

CREATE TABLE Picture (
    id INTEGER NOT NULL,
    workorder_id INTEGER NOT NULL,
    dateiname TEXT,
    beschreibung TEXT,
    logbuch BOOLEAN,
    an_kunde BOOLEAN,
    created_at TIMESTAMP
);

CREATE TABLE Document (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    dateiname TEXT NOT NULL,
    original_name TEXT NOT NULL,
    beschreibung TEXT,
    mime_type TEXT,
    size_bytes BIGINT,
    an_kunde BOOLEAN,
    uploaded_by INTEGER,
    uploaded_at TIMESTAMP
);

CREATE TABLE AcceptanceProtocol (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    sequenz_nr INTEGER NOT NULL,
    vorgaenger_id INTEGER,
    aktiv BOOLEAN,
    gesamt_abgenommen BOOLEAN,
    abnahme_datum DATE,
    abnahme_ort TEXT,
    bemerkungen TEXT,
    nachbesserungstermin DATE,
    unterschrift_kunde TEXT,
    unterschrieben_am TIMESTAMP,
    unterschrieben_von TEXT,
    erstellt_am TIMESTAMP,
    erstellt_von_id INTEGER
);

CREATE TABLE AcceptanceItem (
    id INTEGER NOT NULL,
    abgenommen BOOLEAN,
    bemerkung TEXT,
    bezeichnung TEXT,
    status TEXT,
    sort_order INTEGER
);

CREATE TABLE AcceptanceDefect (
    id INTEGER NOT NULL,
    beschreibung TEXT NOT NULL,
    erfasst_am TIMESTAMP,
    nachbesserung_bis DATE,
    behoben BOOLEAN,
    behoben_am TIMESTAMP,
    intern_status TEXT NOT NULL,
    intern_status_am TIMESTAMP
);

CREATE TABLE AcceptanceTemplate (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    name TEXT NOT NULL,
    is_default BOOLEAN,
    aktiv BOOLEAN,
    erstellt_am TIMESTAMP
);

CREATE TABLE AcceptanceTemplateItem (
    id INTEGER NOT NULL,
    bezeichnung TEXT NOT NULL,
    sort_order INTEGER
);

CREATE TABLE HistoryEntry (
    id INTEGER NOT NULL,
    workorder_id INTEGER NOT NULL,
    aktion TEXT,
    details TEXT,
    created_at TIMESTAMP
);

CREATE TABLE LogbookEntry (
    id INTEGER NOT NULL,
    workorder_id INTEGER,
    datum DATE NOT NULL,
    abfahrt TEXT,
    ankunft TEXT,
    rueckfahrt TEXT,
    zurueck TEXT,
    start_km DOUBLE PRECISION,
    ende_km DOUBLE PRECISION,
    route TEXT,
    zweck TEXT,
    fahrzeug TEXT,
    privat_anteil DOUBLE PRECISION,
    created_at TIMESTAMP
);

CREATE TABLE NumberSequence (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    name TEXT NOT NULL,
    prefix TEXT,
    current INTEGER,
    padding INTEGER
);

CREATE TABLE Setting (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    key TEXT NOT NULL,
    value TEXT,
    label TEXT,
    override_master INTEGER NOT NULL
);

CREATE TABLE CustomerPortalUser (
    id INTEGER NOT NULL,
    username TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    aktiv BOOLEAN,
    must_change_pw BOOLEAN,
    last_login TIMESTAMP,
    created_at TIMESTAMP,
    failed_attempts INTEGER,
    locked_until TIMESTAMP
);

CREATE TABLE PasswordEntry (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    created_by INTEGER,
    gruppe TEXT,
    titel TEXT NOT NULL,
    benutzername TEXT,
    passwort_enc TEXT,
    url TEXT,
    notizen_enc TEXT,
    icon TEXT,
    aktiv BOOLEAN,
    keepass_uid TEXT,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE TABLE TimeSheet (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    source TEXT,
    datum DATE NOT NULL,
    minuten INTEGER,
    erfasst_von TEXT,
    startzeit TEXT,
    endzeit TEXT,
    beschreibung TEXT,
    timer_start TIMESTAMP,
    timer_paused_at TIMESTAMP,
    abgerechnet BOOLEAN,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE TABLE TaxReserve (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    jahr INTEGER NOT NULL,
    monat INTEGER NOT NULL,
    quartal INTEGER,
    typ TEXT,
    erledigt BOOLEAN
);

CREATE TABLE TimesheetActivity (
    id INTEGER NOT NULL,
    beschreibung TEXT NOT NULL,
    created_at TIMESTAMP
);

CREATE TABLE Reminder (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    titel TEXT NOT NULL,
    beschreibung TEXT,
    faellig_am DATE NOT NULL,
    prioritaet TEXT,
    erledigt BOOLEAN,
    erledigt_am TIMESTAMP,
    erstellt_am TIMESTAMP,
    zeit_von TEXT,
    zeit_bis TEXT,
    termin_typ TEXT,
    wiederkehrend BOOLEAN,
    intervall TEXT,
    intervall_tage TEXT,
    intervall_tag INTEGER,
    intervall_monat INTEGER,
    intervall_alle INTEGER,
    token_cancel TEXT,
    cancelled_at TIMESTAMP,
    cancelled_ip TEXT,
    crm_lead_id INTEGER,
    crm_contact_id INTEGER,
    crm_company_id INTEGER,
    crm_task_id INTEGER
);

CREATE TABLE MaintenanceContract (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    titel TEXT NOT NULL,
    beschreibung TEXT,
    intervall TEXT,
    preis_netto DOUBLE PRECISION,
    mwst_satz DOUBLE PRECISION,
    beginn DATE NOT NULL,
    ende DATE,
    letzte_wartung DATE,
    naechste_wartung DATE,
    auto_rechnung BOOLEAN,
    aktiv BOOLEAN,
    notizen TEXT,
    created_at TIMESTAMP
);

CREATE TABLE RecurringInvoice (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    titel TEXT NOT NULL,
    beschreibung TEXT,
    intervall TEXT,
    preis_netto DOUBLE PRECISION,
    mwst_satz DOUBLE PRECISION,
    naechste_ausfuehrung DATE NOT NULL,
    letzte_ausfuehrung DATE,
    auto_versand BOOLEAN,
    aktiv BOOLEAN,
    notizen TEXT,
    created_at TIMESTAMP
);

CREATE TABLE Device (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    kategorie TEXT,
    hersteller TEXT,
    modell TEXT,
    seriennummer TEXT,
    hostname TEXT,
    ip_adresse TEXT,
    mac_adresse TEXT,
    standort TEXT,
    kaufdatum DATE,
    garantie_bis DATE,
    firmware TEXT,
    zugangsdaten TEXT,
    notizen TEXT,
    letzte_wartung DATE,
    status TEXT,
    aktiv BOOLEAN,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE TABLE KummerkastenEntry (
    id INTEGER NOT NULL,
    source TEXT,
    typ TEXT,
    titel TEXT NOT NULL,
    beschreibung TEXT,
    prioritaet TEXT,
    status TEXT,
    admin_kommentar TEXT,
    erstellt_am TIMESTAMP,
    aktualisiert_am TIMESTAMP
);

CREATE TABLE PortalViewState (
    id INTEGER NOT NULL,
    customer_id INTEGER NOT NULL,
    kind TEXT NOT NULL,
    last_seen_at TIMESTAMP
);

CREATE TABLE ReferralLog (
    id INTEGER NOT NULL,
    referral_code TEXT NOT NULL,
    empfaenger_email TEXT,
    empfaenger_name TEXT,
    versendet_am TIMESTAMP,
    versendet_von INTEGER,
    status TEXT,
    converted_at TIMESTAMP,
    converted_manually BOOLEAN,
    converted_note TEXT,
    proposed_by INTEGER,
    proposed_at TIMESTAMP,
    converted_tenant_id INTEGER
);

CREATE TABLE ColdLead (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    firma TEXT NOT NULL,
    ansprechpartner TEXT,
    branche TEXT,
    strasse TEXT,
    plz TEXT,
    ort TEXT,
    land TEXT,
    telefon TEXT,
    mobil TEXT,
    email TEXT,
    webseite TEXT,
    quelle TEXT,
    briefanrede TEXT,
    mitarbeiterzahl TEXT,
    notiz_kurz TEXT,
    newsletter_sperre BOOLEAN,
    newsletter_sperre_grund TEXT,
    newsletter_sperre_am TIMESTAMP,
    source_customer_id INTEGER,
    status TEXT,
    wiedervorlage_am DATE,
    notizen TEXT,
    converted_customer_id INTEGER,
    converted_at TIMESTAMP,
    created_at TIMESTAMP,
    created_by INTEGER,
    updated_at TIMESTAMP
);

CREATE TABLE ColdLeadActivity (
    id INTEGER NOT NULL,
    lead_id INTEGER NOT NULL,
    typ TEXT,
    text TEXT,
    mail_subject TEXT,
    mail_to TEXT,
    status_from TEXT,
    status_to TEXT,
    created_at TIMESTAMP,
    created_by INTEGER
);

CREATE TABLE ColdCampaign (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    beschreibung TEXT,
    status TEXT,
    mail_subject TEXT,
    mail_template_html TEXT,
    created_at TIMESTAMP,
    created_by INTEGER,
    updated_at TIMESTAMP
);

CREATE TABLE ColdCampaignLead (
    id INTEGER NOT NULL,
    campaign_id INTEGER NOT NULL,
    sent_at TIMESTAMP,
    sent_status TEXT,
    sent_error TEXT,
    added_at TIMESTAMP,
    added_by INTEGER
);

CREATE TABLE ServicePackage (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    beschreibung TEXT,
    rechnungs_titel TEXT,
    rechnungs_text TEXT,
    preis_monthly DOUBLE PRECISION,
    preis_quarterly DOUBLE PRECISION,
    preis_half_yearly DOUBLE PRECISION,
    preis_yearly DOUBLE PRECISION,
    mwst_satz DOUBLE PRECISION,
    free_months_default INTEGER,
    aktiv BOOLEAN,
    sort_order INTEGER,
    created_at TIMESTAMP,
    with_mail_templates BOOLEAN,
    with_demo_data BOOLEAN,
    required_server_type TEXT
);

CREATE TABLE RentedServer (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    provider TEXT,
    hostname TEXT,
    ip_address TEXT,
    woa_url TEXT,
    dns_eintrag TEXT,
    ssh_user TEXT,
    ssh_port INTEGER,
    miete_netto DOUBLE PRECISION,
    miete_intervall TEXT,
    ssh_password_enc TEXT,
    root_password_enc TEXT,
    notizen_enc TEXT,
    master_api_url TEXT,
    master_api_token TEXT,
    verify_ssl BOOLEAN NOT NULL,
    erp_ocr_token TEXT,
    last_sync_at TIMESTAMP,
    last_sync_status TEXT,
    last_sync_message TEXT,
    last_software_version TEXT,
    last_health_at TIMESTAMP,
    last_health_payload TEXT,
    sa_username TEXT,
    sa_password_enc TEXT,
    installed_at TIMESTAMP,
    tenant_slug_remote TEXT,
    is_master BOOLEAN,
    aktiv BOOLEAN,
    created_at TIMESTAMP,
    server_type TEXT,
    max_tenants INTEGER,
    tenant_count INTEGER,
    tenant_count_at TIMESTAMP,
    push_failure_count INTEGER NOT NULL,
    push_last_failure_at TIMESTAMP,
    push_last_error TEXT,
    push_disabled BOOLEAN NOT NULL,
    push_disabled_at TIMESTAMP
);

CREATE TABLE ServiceContract (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    intervall TEXT,
    vertrag_start DATE NOT NULL,
    rechnung_ab DATE NOT NULL,
    naechste_rechnung DATE NOT NULL,
    individualpreis_netto DOUBLE PRECISION,
    rabatt_prozent DOUBLE PRECISION,
    free_months_remaining INTEGER,
    credit_eur DOUBLE PRECISION,
    bonus_pending_eur DOUBLE PRECISION,
    sales_partner_id INTEGER,
    provision_pct DOUBLE PRECISION,
    commission_until DATE,
    auto_versand BOOLEAN,
    aktiv BOOLEAN,
    gekuendigt_am DATE,
    geloescht_am TIMESTAMP,
    notizen TEXT,
    created_at TIMESTAMP,
    last_invoice_at TIMESTAMP,
    last_invoice_workorder_id INTEGER,
    shop_saas_config_id INTEGER,
    prorata_pending BOOLEAN NOT NULL
);

CREATE TABLE SalesPartner (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    tier TEXT NOT NULL,
    provision_pct DOUBLE PRECISION NOT NULL,
    aktiv BOOLEAN NOT NULL,
    firma TEXT,
    ansprechpartner TEXT,
    email TEXT,
    telefon TEXT,
    is_service_techniker BOOLEAN NOT NULL,
    strasse TEXT,
    plz TEXT,
    ort TEXT,
    ustid TEXT,
    steuer_status TEXT,
    iban TEXT,
    bic TEXT,
    bank_name TEXT,
    commission_months INTEGER,
    aktiv_ab DATE,
    aktiv_bis DATE,
    notizen TEXT,
    vertrag_pdf_path TEXT,
    vertrag_versendet_am TIMESTAMP,
    vertrag_versand_method TEXT,
    vertrag_signatur_token TEXT,
    vertrag_signiertes_pdf_path TEXT,
    vertrag_signiert_am TIMESTAMP,
    vertrag_signatur_data TEXT,
    vertrag_signatur_ip TEXT,
    vertrag_signatur_user_agent TEXT,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE TABLE PartnerCommission (
    id INTEGER NOT NULL,
    payout_id INTEGER,
    tenant_id INTEGER NOT NULL,
    basis_netto DOUBLE PRECISION NOT NULL,
    provision_pct DOUBLE PRECISION NOT NULL,
    betrag_netto DOUBLE PRECISION NOT NULL,
    status TEXT NOT NULL,
    pending_at TIMESTAMP,
    earned_at TIMESTAMP,
    paid_at DATE,
    paid_workorder_id INTEGER,
    cancelled_at TIMESTAMP,
    cancelled_reason TEXT,
    notes TEXT,
    created_at TIMESTAMP
);

CREATE TABLE PartnerPayout (
    id INTEGER NOT NULL,
    payout_nr TEXT,
    tenant_id INTEGER NOT NULL,
    paid_at DATE NOT NULL,
    summe_netto DOUBLE PRECISION NOT NULL,
    summe_ust DOUBLE PRECISION,
    summe_brutto DOUBLE PRECISION,
    ust_satz DOUBLE PRECISION,
    steuer_status_snapshot TEXT,
    note TEXT,
    pdf_path TEXT,
    pdf_generated_at TIMESTAMP,
    mail_sent_at TIMESTAMP,
    mail_sent_to TEXT,
    created_by INTEGER,
    created_at TIMESTAMP
);

CREATE TABLE ContractSetup (
    id INTEGER NOT NULL,
    setup_phase TEXT NOT NULL,
    partner_decision TEXT,
    progress_pct INTEGER,
    subdomain TEXT,
    server_ip TEXT,
    dns_recipient TEXT,
    dns_bcc TEXT,
    dns_mail_sent_at TIMESTAMP,
    dns_confirmed_at TIMESTAMP,
    dns_confirmed_by INTEGER,
    server_ready_at TIMESTAMP,
    server_ready_by INTEGER,
    server_notes TEXT,
    ssh_host TEXT,
    ssh_port INTEGER,
    ssh_user TEXT,
    package_uploaded_at TIMESTAMP,
    package_path_remote TEXT,
    package_filename TEXT,
    target_tenant_id INTEGER,
    target_tenant_name TEXT,
    target_tenant_slug TEXT,
    target_branche TEXT,
    welcome_recipient TEXT,
    welcome_sent_at TIMESTAMP,
    admin_username TEXT,
    onboarding_recipient TEXT,
    onboarding_bcc TEXT,
    onboarding_sent_at TIMESTAMP,
    onboarding_csv_attached BOOLEAN,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE TABLE ContractSetupHistory (
    id INTEGER NOT NULL,
    contract_id INTEGER NOT NULL,
    action TEXT NOT NULL,
    phase_from TEXT,
    phase_to TEXT,
    user_id INTEGER,
    notes TEXT,
    created_at TIMESTAMP
);

CREATE TABLE AppVersion (
    id INTEGER NOT NULL,
    version TEXT NOT NULL,
    notes TEXT,
    set_by INTEGER,
    set_at TIMESTAMP,
    is_current BOOLEAN,
    change_pdf_path TEXT,
    change_pdf_sha256 TEXT,
    change_pdf_size INTEGER,
    audit_detail_pdf_path TEXT,
    audit_detail_pdf_sha256 TEXT,
    audit_detail_pdf_size INTEGER,
    audit_layperson_pdf_path TEXT,
    audit_layperson_pdf_sha256 TEXT,
    audit_layperson_pdf_size INTEGER,
    audit_run_id INTEGER
);

CREATE TABLE IpBlacklist (
    id INTEGER NOT NULL,
    ip TEXT NOT NULL,
    grund TEXT,
    aktiv BOOLEAN,
    auto BOOLEAN,
    expires_at TIMESTAMP,
    created_at TIMESTAMP,
    created_by INTEGER,
    last_hit_at TIMESTAMP,
    hit_count INTEGER,
    safe_marked_by INTEGER,
    safe_marked_at TIMESTAMP,
    safe_reason TEXT,
    origin_server TEXT
);

CREATE TABLE LoginAudit (
    id INTEGER NOT NULL,
    username TEXT,
    ip TEXT,
    user_agent TEXT,
    success BOOLEAN,
    reason TEXT,
    created_at TIMESTAMP
);

CREATE TABLE ScopeAuditBlock (
    id INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL,
    actor_user_id INTEGER,
    actor_username TEXT,
    actor_tenant_id INTEGER,
    actor_is_admin BOOLEAN,
    actor_is_superadmin BOOLEAN,
    target_model TEXT,
    target_id INTEGER,
    target_tenant_id INTEGER,
    route TEXT,
    method TEXT,
    reason TEXT,
    ip TEXT,
    user_agent TEXT
);

CREATE TABLE SecurityAudit (
    id INTEGER NOT NULL,
    app_version TEXT,
    status TEXT,
    report_mode TEXT,
    audited_by INTEGER,
    audited_by_name TEXT,
    audited_at TIMESTAMP,
    approved_by INTEGER,
    approved_by_name TEXT,
    approved_at TIMESTAMP,
    results_json TEXT,
    overall_status TEXT,
    pass_count INTEGER,
    fail_count INTEGER,
    warn_count INTEGER,
    skip_count INTEGER,
    auditor_notes TEXT,
    version_history_json TEXT,
    signature_hash TEXT,
    signed_at TIMESTAMP,
    pdf_path TEXT,
    pdf_sha256 TEXT,
    pdf_size INTEGER,
    sa_notified_at TIMESTAMP,
    created_at TIMESTAMP
);

CREATE TABLE SyncHistory (
    id INTEGER NOT NULL,
    rented_server_id INTEGER NOT NULL,
    started_at TIMESTAMP NOT NULL,
    finished_at TIMESTAMP,
    action TEXT,
    status TEXT,
    files_changed INTEGER,
    backup_path TEXT,
    error_message TEXT,
    log_excerpt TEXT,
    triggered_by INTEGER
);

CREATE TABLE UpdateJob (
    id INTEGER NOT NULL,
    status TEXT,
    started_at TIMESTAMP NOT NULL,
    finished_at TIMESTAMP,
    last_heartbeat TIMESTAMP NOT NULL,
    current_step TEXT,
    progress_percent INTEGER,
    log_excerpt TEXT,
    error_message TEXT,
    triggered_by INTEGER,
    cancel_requested BOOLEAN NOT NULL
);

CREATE TABLE UpdateSnapshot (
    id INTEGER NOT NULL,
    snapshot_file TEXT NOT NULL,
    snapshot_size BIGINT,
    version_before TEXT,
    version_after TEXT,
    status TEXT,
    log_excerpt TEXT,
    created_at TIMESTAMP NOT NULL,
    restored_at TIMESTAMP,
    rollback_note TEXT
);

CREATE TABLE TerminVorschlag (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    project_id INTEGER,
    defect_id INTEGER,
    titel TEXT NOT NULL,
    beschreibung TEXT,
    termin_typ TEXT,
    vorschlaege_json TEXT,
    status TEXT,
    accepted_index INTEGER,
    accepted_reminder_id INTEGER,
    accepted_at TIMESTAMP,
    accepted_ip TEXT,
    token_slot1 TEXT,
    token_slot2 TEXT,
    token_slot3 TEXT,
    token_decline TEXT NOT NULL,
    token_cancel TEXT,
    cancelled_at TIMESTAMP,
    cancelled_ip TEXT,
    expires_at TIMESTAMP NOT NULL,
    erstellt_am TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE TABLE TerminVorschlagBlocker (
    id INTEGER NOT NULL,
    slot_index INTEGER NOT NULL,
    datum DATE NOT NULL,
    zeit_von TEXT,
    zeit_bis TEXT,
    caldav_uid TEXT NOT NULL,
    pushed_at TIMESTAMP,
    deleted_at TIMESTAMP,
    erstellt_am TIMESTAMP
);

CREATE TABLE HandbookFeature (
    id INTEGER NOT NULL,
    titel TEXT NOT NULL,
    beschreibung TEXT,
    kategorie TEXT,
    version TEXT,
    reihenfolge INTEGER,
    aktiv BOOLEAN,
    datum DATE,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE TABLE BrancheTemplate (
    id INTEGER NOT NULL,
    branche TEXT NOT NULL,
    label TEXT,
    html_content TEXT,
    aktiv BOOLEAN,
    erstellt_am TIMESTAMP,
    aktualisiert_am TIMESTAMP
);

CREATE TABLE ServiceContractItem (
    id INTEGER NOT NULL,
    sort_order INTEGER,
    pos_typ TEXT,
    titel TEXT,
    beschreibung TEXT,
    intervall TEXT,
    preis_netto DOUBLE PRECISION,
    menge DOUBLE PRECISION,
    mwst_satz DOUBLE PRECISION,
    rabatt_typ TEXT,
    rabatt_wert DOUBLE PRECISION,
    rabatt_bis DATE,
    rabatt_grund TEXT,
    aktiv_ab DATE,
    aktiv_bis DATE,
    abgerechnet_bis DATE,
    sofort_rechnung BOOLEAN,
    aktiv BOOLEAN,
    created_at TIMESTAMP
);

CREATE TABLE ContractBonus (
    id INTEGER NOT NULL,
    bonus_typ TEXT NOT NULL,
    wert DOUBLE PRECISION,
    grund TEXT,
    aktiv_ab DATE,
    aktiv_bis DATE,
    verbraucht BOOLEAN,
    verbraucht_am TIMESTAMP,
    verbraucht_in_workorder_id INTEGER,
    promo_code_id INTEGER,
    aktiv BOOLEAN,
    created_at TIMESTAMP,
    created_by INTEGER
);

CREATE TABLE PromoCode (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    code TEXT NOT NULL,
    titel TEXT,
    beschreibung TEXT,
    code_typ TEXT,
    wert DOUBLE PRECISION,
    gueltig_ab DATE,
    gueltig_bis DATE,
    max_einloesungen INTEGER,
    aktuelle_einloesungen INTEGER,
    nur_neukunden BOOLEAN,
    min_vertragswert_netto DOUBLE PRECISION,
    bonus_aktiv_monate INTEGER,
    aktiv BOOLEAN,
    created_at TIMESTAMP,
    created_by INTEGER
);

CREATE TABLE OnboardingTemplate (
    id INTEGER NOT NULL,
    kind TEXT NOT NULL,
    label TEXT,
    subject TEXT,
    html_content TEXT,
    plain_body TEXT,
    aktiv BOOLEAN,
    erstellt_am TIMESTAMP,
    aktualisiert_am TIMESTAMP
);

CREATE TABLE PartnerContractTemplate (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    label TEXT,
    html_content TEXT,
    aktiv BOOLEAN,
    erstellt_am TIMESTAMP,
    aktualisiert_am TIMESTAMP
);

CREATE TABLE PortalAutoLoginToken (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    customer_portal_user_id INTEGER NOT NULL,
    workorder_id INTEGER,
    token TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    last_used_at TIMESTAMP,
    revoked_at TIMESTAMP,
    use_count INTEGER NOT NULL,
    created_by TEXT,
    scope TEXT,
    permanent BOOLEAN
);

CREATE TABLE MailTemplate (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    kind TEXT NOT NULL,
    subject TEXT NOT NULL,
    html_body TEXT NOT NULL,
    plain_body TEXT,
    updated_at TIMESTAMP NOT NULL,
    updated_by_user_id INTEGER
);

CREATE TABLE TresorPwResetToken (
    id INTEGER NOT NULL,
    customer_id INTEGER NOT NULL,
    token TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    used_at TIMESTAMP,
    created_by TEXT
);

CREATE TABLE LegacyRouteKey (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    key_data TEXT NOT NULL,
    retired_at TIMESTAMP NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    note TEXT
);

CREATE TABLE RevokedToken (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    token_hash TEXT NOT NULL,
    revoked_at TIMESTAMP NOT NULL,
    revoked_by_uid INTEGER,
    reason TEXT
);

CREATE TABLE GeoblockSetting (
    id INTEGER NOT NULL,
    country_code TEXT NOT NULL,
    country_name TEXT NOT NULL,
    continent TEXT,
    blocked BOOLEAN NOT NULL,
    updated_at TIMESTAMP,
    updated_by INTEGER
);

CREATE TABLE GeoblockAllowIP (
    id INTEGER NOT NULL,
    cidr TEXT NOT NULL,
    note TEXT,
    created_at TIMESTAMP,
    created_by INTEGER
);

CREATE TABLE GeoblockStat (
    id INTEGER NOT NULL,
    country_code TEXT NOT NULL,
    stat_date DATE NOT NULL,
    block_count INTEGER NOT NULL
);

CREATE TABLE ClaudeLesson (
    id INTEGER NOT NULL,
    session_date DATE NOT NULL,
    category TEXT NOT NULL,
    severity TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    tags TEXT,
    related_patches TEXT,
    is_active BOOLEAN,
    created_at TIMESTAMP,
    created_by TEXT
);

CREATE TABLE ClaudeStaticSection (
    id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    section_key TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    has_timestamp_placeholder BOOLEAN,
    is_active BOOLEAN,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE TABLE TaxRate (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    prozent DOUBLE PRECISION NOT NULL,
    aktiv BOOLEAN NOT NULL,
    is_default BOOLEAN NOT NULL,
    sort_order INTEGER NOT NULL,
    created_at TIMESTAMP
);

CREATE TABLE ShiftTicket (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    token TEXT NOT NULL,
    valid_until TIMESTAMP NOT NULL,
    created_at TIMESTAMP,
    created_by_id INTEGER,
    used_at TIMESTAMP
);

CREATE TABLE ErpLegalProfile (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    rechtsform TEXT,
    festschreib_periode TEXT,
    kleinunternehmer BOOLEAN,
    besteuerungsart TEXT,
    buchen_belegpflicht BOOLEAN,
    lager_fibu_methode TEXT,
    lager_wareneinsatz_konto TEXT,
    bank_sachkonto_auto_pi BOOLEAN,
    customer_bank_autosync TEXT,
    handelsregister TEXT,
    steuernummer TEXT,
    ust_id TEXT,
    default_currency TEXT,
    currency_custom_code TEXT,
    currency_custom_symbol TEXT,
    datev_beraternr TEXT,
    datev_mandantnr TEXT,
    datev_wj_beginn INTEGER,
    erp_startdatum DATE,
    legacy_source TEXT,
    legacy_object_id TEXT,
    created_at TIMESTAMP,
    updated_at TIMESTAMP,
    imap_server TEXT,
    imap_port INTEGER,
    imap_user TEXT,
    imap_password TEXT,
    imap_folder TEXT,
    imap_ssl BOOLEAN,
    imap_aktiv BOOLEAN,
    imap_forward_to TEXT,
    imap_interval INTEGER,
    gewerbesteuer_hebesatz INTEGER,
    erechnung_format TEXT
);

CREATE TABLE ErpSupplierIban (
    id INTEGER NOT NULL,
    supplier_id INTEGER NOT NULL,
    iban TEXT NOT NULL,
    bic TEXT,
    notiz TEXT,
    created_at TIMESTAMP
);

CREATE TABLE ErpFintsInstitute (
    blz TEXT NOT NULL,
    bic TEXT,
    institut TEXT,
    ort TEXT,
    rz TEXT,
    organisation TEXT,
    hbci_dns TEXT,
    hbci_version TEXT,
    pintan_url TEXT,
    fints_version TEXT,
    updated_at_src DATE,
    imported_at TIMESTAMP
);

CREATE TABLE ErpExchangeRate (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    from_currency TEXT,
    to_currency TEXT,
    rate NUMERIC,
    valid_from DATE,
    valid_until DATE,
    source TEXT,
    created_at TIMESTAMP
);

CREATE TABLE ErpEstTarifParams (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    jahr INTEGER NOT NULL,
    grundfreibetrag INTEGER,
    zone2_bis INTEGER,
    zone3_bis INTEGER,
    zone4_bis INTEGER,
    z2a TEXT,
    z2b TEXT,
    z3a TEXT,
    z3b TEXT,
    z3c TEXT,
    z4_abzug TEXT,
    z5_abzug TEXT,
    soli_freigrenze INTEGER,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE TABLE ErpKuGrenzen (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    jahr INTEGER NOT NULL,
    grenze_vorjahr INTEGER,
    grenze_laufend INTEGER,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE TABLE ErpLedgerLock (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    buchungsperiode TEXT NOT NULL,
    status TEXT,
    locked_at TIMESTAMP,
    locked_by_user_id INTEGER,
    hash_snapshot TEXT,
    created_at TIMESTAMP
);

CREATE TABLE ErpAuditTrail (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    seq_no INTEGER NOT NULL,
    entity TEXT NOT NULL,
    entity_id INTEGER,
    aktion TEXT NOT NULL,
    user_id INTEGER,
    ip TEXT,
    user_agent TEXT,
    freitext TEXT,
    before_hash TEXT,
    after_hash TEXT,
    created_at TIMESTAMP
);

CREATE TABLE ErpChartOfAccounts (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    rahmen TEXT NOT NULL,
    aktiv BOOLEAN,
    gesperrt_ab TIMESTAMP,
    created_at TIMESTAMP
);

CREATE TABLE ErpAccount (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    rahmen TEXT NOT NULL,
    kontonummer TEXT NOT NULL,
    bezeichnung TEXT NOT NULL,
    kontoart TEXT,
    kontenklasse INTEGER,
    steuer_relevant BOOLEAN,
    ust_kennzeichen TEXT,
    automatik_konto BOOLEAN,
    gesperrt BOOLEAN,
    sort_order INTEGER,
    currency TEXT,
    legacy_source TEXT,
    legacy_object_id TEXT,
    created_at TIMESTAMP
);

CREATE TABLE ErpCostCenter (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    nummer TEXT NOT NULL,
    bezeichnung TEXT,
    aktiv BOOLEAN,
    parent_id INTEGER,
    created_at TIMESTAMP
);

CREATE TABLE ErpFiscalYear (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    jahr INTEGER NOT NULL,
    beginn DATE NOT NULL,
    ende DATE NOT NULL,
    status TEXT,
    created_at TIMESTAMP
);

CREATE TABLE ErpPeriod (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    fiscal_year_id INTEGER,
    periode_key TEXT NOT NULL,
    bezeichnung TEXT,
    status TEXT,
    created_at TIMESTAMP
);

CREATE TABLE ErpTaxAccountMap (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    rahmen TEXT NOT NULL,
    ust_kennzeichen TEXT NOT NULL,
    beschreibung TEXT,
    ust_konto TEXT,
    vst_konto TEXT,
    steuer_konto TEXT,
    prozent NUMERIC,
    gueltig_ab DATE,
    gueltig_bis DATE,
    created_at TIMESTAMP
);

CREATE TABLE ErpJournalEntry (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    belegnummer TEXT NOT NULL,
    belegdatum DATE NOT NULL,
    buchungsdatum DATE NOT NULL,
    buchungstext TEXT,
    erfasst_von_user_id INTEGER,
    festgeschrieben BOOLEAN,
    festgeschrieben_am TIMESTAMP,
    storno_of_id INTEGER,
    herkunft TEXT,
    herkunft_ref_id INTEGER,
    currency TEXT,
    created_at TIMESTAMP
);

CREATE TABLE ErpJournalLine (
    id INTEGER NOT NULL,
    entry_id INTEGER NOT NULL,
    konto TEXT NOT NULL,
    gegenkonto TEXT,
    soll_betrag NUMERIC,
    haben_betrag NUMERIC,
    steuer_betrag NUMERIC,
    ust_kennzeichen TEXT,
    kostenstelle_id INTEGER,
    zeilentext TEXT
);

CREATE TABLE ErpDebitorAccount (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    customer_id INTEGER NOT NULL,
    kontonummer TEXT NOT NULL,
    created_at TIMESTAMP
);

CREATE TABLE ErpSupplier (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    name TEXT NOT NULL,
    anschrift TEXT,
    ust_id TEXT,
    steuernummer TEXT,
    iban TEXT,
    bic TEXT,
    zahlungsziel INTEGER,
    skonto_tage INTEGER,
    skonto_prozent DOUBLE PRECISION,
    kreditorenkonto TEXT,
    aufwandskonto_default TEXT,
    ansprechpartner TEXT,
    ap_position TEXT,
    telefon TEXT,
    email TEXT,
    website TEXT,
    strasse TEXT,
    plz TEXT,
    ort TEXT,
    land TEXT,
    unsere_kundennr TEXT,
    notizen TEXT,
    lieferzeit_tage INTEGER,
    mindestbestellwert NUMERIC,
    aktiv BOOLEAN,
    created_at TIMESTAMP
);

CREATE TABLE ErpPurchaseInvoice (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    rechnungsnr TEXT NOT NULL,
    belegdatum DATE NOT NULL,
    eingangsdatum DATE,
    faellig_am DATE,
    netto NUMERIC,
    steuer NUMERIC,
    brutto NUMERIC,
    currency TEXT,
    aufwandskonto TEXT,
    buchungstext TEXT,
    split_buchung TEXT,
    geprueft BOOLEAN,
    geprueft_von INTEGER,
    geprueft_am TIMESTAMP,
    dokument_pfad TEXT,
    status TEXT,
    bezahlt BOOLEAN,
    bezahlt_am DATE,
    journal_entry_id INTEGER,
    created_at TIMESTAMP,
    skontofrist INTEGER,
    skontosatz NUMERIC
);

CREATE TABLE ErpSupplierArticle (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    lieferanten_artikelnr TEXT,
    ek_preis NUMERIC,
    currency TEXT,
    mindestmenge INTEGER,
    staffelpreise TEXT,
    lieferzeit_tage INTEGER,
    ist_hauptlieferant BOOLEAN,
    aktiv BOOLEAN,
    created_at TIMESTAMP
);

CREATE TABLE ErpPurchaseOrder (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    bestellnummer TEXT NOT NULL,
    status TEXT,
    bestelldatum DATE,
    liefertermin DATE,
    netto_summe NUMERIC,
    steuer_summe NUMERIC,
    brutto_summe NUMERIC,
    currency TEXT,
    notizen TEXT,
    purchase_invoice_id INTEGER,
    erstellt_von INTEGER,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE TABLE ErpPurchaseOrderLine (
    id INTEGER NOT NULL,
    purchase_order_id INTEGER NOT NULL,
    artikelnr TEXT,
    bezeichnung TEXT,
    menge DOUBLE PRECISION NOT NULL,
    ek_preis NUMERIC,
    steuersatz NUMERIC,
    currency TEXT,
    geliefert_menge DOUBLE PRECISION,
    notiz TEXT
);

CREATE TABLE ErpMaterialBedarf (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    menge_position DOUBLE PRECISION,
    menge_bestand DOUBLE PRECISION,
    menge_bedarf DOUBLE PRECISION,
    menge_bestellt DOUBLE PRECISION,
    menge_geliefert DOUBLE PRECISION,
    einheit TEXT,
    best_ek NUMERIC,
    status TEXT,
    workorder_nr TEXT,
    artikel_bez TEXT,
    created_at TIMESTAMP,
    created_by INTEGER,
    notiz TEXT
);

CREATE TABLE ErpCashRegister (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    name TEXT,
    sachkonto TEXT,
    currency TEXT,
    aktiv BOOLEAN,
    anfangssaldo NUMERIC,
    created_at TIMESTAMP
);

CREATE TABLE ErpCashEntry (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    lfd_nr INTEGER NOT NULL,
    datum DATE NOT NULL,
    vorgang TEXT NOT NULL,
    betrag NUMERIC NOT NULL,
    gegenkonto TEXT,
    zweck TEXT,
    erfasst_von_user_id INTEGER,
    festgeschrieben BOOLEAN,
    storno_of_id INTEGER,
    journal_entry_id INTEGER,
    created_at TIMESTAMP
);

CREATE TABLE ErpDocument (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    original_name TEXT NOT NULL,
    stored_name TEXT NOT NULL,
    mime_type TEXT,
    size_bytes BIGINT,
    integrity_hash TEXT,
    belegtyp TEXT,
    beleg_status TEXT,
    steuerberater_relevant BOOLEAN,
    beschreibung TEXT,
    customer_id INTEGER,
    purchase_invoice_id INTEGER,
    asset_id INTEGER,
    journal_entry_id INTEGER,
    legacy_source TEXT,
    legacy_object_id TEXT,
    legacy_tree_id TEXT,
    legacy_filename TEXT,
    legacy_imported_at TIMESTAMP,
    uploaded_by INTEGER,
    uploaded_at TIMESTAMP,
    kategorie_id INTEGER,
    periode TEXT,
    tags TEXT,
    beleg_datum DATE,
    lieferant_id INTEGER,
    workorder_id INTEGER,
    bank_tx_id INTEGER,
    ocr_status TEXT
);

CREATE TABLE ErpDocumentFulltext (
    id INTEGER NOT NULL,
    quelle TEXT,
    text TEXT,
    indexed_at TIMESTAMP
);

CREATE TABLE ErpDmsKategorie (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    parent_id INTEGER,
    name TEXT NOT NULL,
    beschreibung TEXT,
    icon TEXT,
    sort_order INTEGER,
    aktiv BOOLEAN,
    created_at TIMESTAMP
);

CREATE TABLE ErpDmsAudit (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    document_id INTEGER NOT NULL,
    user_id INTEGER,
    aktion TEXT NOT NULL,
    von_status TEXT,
    zu_status TEXT,
    kommentar TEXT,
    created_at TIMESTAMP
);

CREATE TABLE SambaShare (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    comment TEXT,
    browseable BOOLEAN,
    aktiv BOOLEAN,
    created_at TIMESTAMP
);

CREATE TABLE SambaShareAcl (
    id INTEGER NOT NULL,
    can_read BOOLEAN,
    can_write BOOLEAN
);

CREATE TABLE ScanRequest (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    user_id INTEGER NOT NULL,
    ziel_typ TEXT NOT NULL,
    ziel_id INTEGER,
    status TEXT,
    document_id INTEGER,
    created_at TIMESTAMP,
    completed_at TIMESTAMP,
    expires_at TIMESTAMP,
    hinweis TEXT
);

CREATE TABLE EinsatzSession (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    pairing_code TEXT NOT NULL,
    qr_token TEXT,
    code_expires_at TIMESTAMP NOT NULL,
    session_token TEXT,
    user_id INTEGER,
    customer_id INTEGER,
    reminder_id INTEGER,
    workorder_id INTEGER,
    scope TEXT,
    status TEXT,
    paired_at TIMESTAMP,
    expires_at TIMESTAMP,
    last_activity TIMESTAMP,
    ended_at TIMESTAMP,
    ended_by TEXT,
    browser_fp TEXT,
    ip_address TEXT,
    user_agent TEXT,
    created_at TIMESTAMP
);

CREATE TABLE ErpSerialCharge (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    typ TEXT,
    nummer TEXT NOT NULL,
    mhd DATE,
    status TEXT,
    workorder_id INTEGER,
    movement_id INTEGER,
    created_at TIMESTAMP
);

CREATE TABLE ErpEinvoiceImport (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    status TEXT,
    filename TEXT,
    dateityp TEXT,
    raw_xml TEXT,
    profil TEXT,
    format_typ TEXT,
    lieferant_name TEXT,
    lieferant_ustid TEXT,
    lieferant_iban TEXT,
    rechnungsnr TEXT,
    rechnungsdatum DATE,
    faelligkeit DATE,
    waehrung TEXT,
    netto_betrag NUMERIC,
    steuer_betrag NUMERIC,
    brutto_betrag NUMERIC,
    steuer_prozent NUMERIC,
    verwendungszweck TEXT,
    positionen_json TEXT,
    purchase_invoice_id INTEGER,
    document_id INTEGER,
    fehler TEXT,
    warnungen TEXT,
    hash_dedup TEXT,
    created_at TIMESTAMP,
    created_by INTEGER,
    confirmed_at TIMESTAMP,
    confirmed_by INTEGER
);

CREATE TABLE ErpSupplierCsvMapping (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    name TEXT,
    trennzeichen TEXT,
    encoding TEXT,
    header_zeile INTEGER,
    skip_zeilen INTEGER,
    col_ean TEXT,
    col_artikelnr TEXT,
    col_bezeichnung TEXT,
    col_ek_preis TEXT,
    col_mindestmenge TEXT,
    col_einheit TEXT,
    col_hersteller TEXT,
    col_hersteller_anr TEXT,
    col_bild_url TEXT,
    col_kategorie TEXT,
    col_unterkategorie TEXT,
    fuzzy_schwelle INTEGER,
    auto_anlegen BOOLEAN,
    auto_kategorie TEXT,
    auto_update_url TEXT,
    auto_update_aktiv BOOLEAN,
    auto_update_auth_type TEXT,
    auto_update_auth_user TEXT,
    auto_update_auth_pass TEXT,
    auto_update_auth_header TEXT,
    auto_update_omd_token_url TEXT,
    auto_update_omd_client_id TEXT,
    auto_update_omd_client_secret TEXT,
    auto_update_omd_cred_location TEXT,
    auto_update_omd_customer_id TEXT,
    auto_update_intervall INTEGER,
    auto_update_typ TEXT,
    auto_update_uhrzeit TEXT,
    auto_update_wochentage TEXT,
    auto_update_next_run TIMESTAMP,
    auto_inaktiv_setzen BOOLEAN,
    auto_update_last_status TEXT,
    auto_update_last_error TEXT,
    auto_update_last_run TIMESTAMP,
    last_import_at TIMESTAMP,
    last_import_count INTEGER,
    created_at TIMESTAMP
);

CREATE TABLE ErpStockMovement (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    datum DATE NOT NULL,
    bewegungsart TEXT,
    menge DOUBLE PRECISION NOT NULL,
    ek_preis DOUBLE PRECISION,
    currency TEXT,
    herkunft TEXT,
    herkunft_ref_id INTEGER,
    notiz TEXT,
    erfasst_von INTEGER,
    erfasst_am TIMESTAMP,
    mitarbeiter_id INTEGER,
    supplier_id INTEGER,
    gutschrift_nr TEXT,
    schadensgrund TEXT
);

CREATE TABLE ErpAsset (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    inventarnr TEXT,
    bezeichnung TEXT NOT NULL,
    anlagenklasse TEXT,
    anschaffungsdatum DATE NOT NULL,
    anschaffungskosten NUMERIC NOT NULL,
    currency TEXT,
    nutzungsdauer_jahre INTEGER,
    afa_methode TEXT,
    anlagenkonto TEXT,
    afa_konto TEXT,
    kum_afa NUMERIC,
    letztes_afa_jahr INTEGER,
    aktiv BOOLEAN,
    abgang_datum DATE,
    abgang_art TEXT,
    device_id INTEGER,
    kostenstelle_id INTEGER,
    created_at TIMESTAMP
);

CREATE TABLE ErpBankAccount (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    name TEXT,
    iban TEXT,
    bic TEXT,
    sachkonto TEXT,
    aktiv BOOLEAN,
    kontotyp TEXT,
    ist_hauptkonto BOOLEAN,
    anfangssaldo NUMERIC,
    fints_aktiv BOOLEAN,
    fints_blz TEXT,
    fints_endpoint_url TEXT,
    fints_login TEXT,
    fints_pin_encrypted TEXT,
    fints_tan_methode TEXT,
    fints_letzter_abruf TIMESTAMP,
    fints_status TEXT,
    fints_letzter_fehler TEXT,
    fints_system_id TEXT,
    created_at TIMESTAMP
);

CREATE TABLE ErpBankTransaction (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    bank_account_id INTEGER,
    buchungsdatum DATE,
    betrag NUMERIC NOT NULL,
    verwendungszweck TEXT,
    gegenname TEXT,
    gegen_iban TEXT,
    status TEXT,
    match_wo_id INTEGER,
    match_pi_id INTEGER,
    match_type TEXT,
    match_transfer_tx_id INTEGER,
    match_sachkonto TEXT,
    match_steuersatz INTEGER,
    import_hash TEXT,
    created_at TIMESTAMP,
    document_id INTEGER,
    match_darlehen_id INTEGER,
    match_split_data TEXT,
    match_abschlag_id INTEGER,
    match_konto_override TEXT,
    hinweis_text TEXT
);

CREATE TABLE ErpDarlehen (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    darlehensnr TEXT,
    bezeichnung TEXT,
    glaeubiger TEXT,
    glaeubiger_iban TEXT,
    darlehensbetrag_brutto NUMERIC,
    restschuld NUMERIC,
    zinssatz NUMERIC,
    startdatum DATE,
    enddatum_geplant DATE,
    monatsrate_soll NUMERIC,
    konto_darlehen TEXT,
    konto_zinsen TEXT,
    aktiv BOOLEAN,
    notiz TEXT,
    created_at TIMESTAMP
);

CREATE TABLE ErpAbschlagskonto (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    bezeichnung TEXT,
    anbieter TEXT,
    anbieter_iban TEXT,
    vertragsnummer TEXT,
    art TEXT,
    monatlicher_abschlag NUMERIC,
    konto_aufwand TEXT,
    ust_satz INTEGER,
    konto_aufwand_2 TEXT,
    anteil_aufwand_2_proz INTEGER,
    aktiv BOOLEAN,
    notiz TEXT,
    created_at TIMESTAMP
);

CREATE TABLE ErpPaymentRun (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    ausfuehrungsdatum DATE NOT NULL,
    status TEXT,
    summe NUMERIC,
    anzahl INTEGER,
    currency TEXT,
    xml_exported BOOLEAN,
    created_at TIMESTAMP,
    created_by_id INTEGER
);

CREATE TABLE ErpPaymentRunItem (
    id INTEGER NOT NULL,
    run_id INTEGER NOT NULL,
    purchase_invoice_id INTEGER,
    empfaenger TEXT NOT NULL,
    iban TEXT NOT NULL,
    bic TEXT,
    betrag NUMERIC NOT NULL,
    currency TEXT,
    verwendungszweck TEXT
);

CREATE TABLE ErpVatReturn (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    jahr INTEGER NOT NULL,
    monat INTEGER NOT NULL,
    kz81 NUMERIC,
    kz86 NUMERIC,
    kz66 NUMERIC,
    ust_19 NUMERIC,
    ust_7 NUMERIC,
    zahllast NUMERIC,
    status TEXT,
    berechnet_am TIMESTAMP,
    created_at TIMESTAMP
);

CREATE TABLE ErpPreisSchema (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    name TEXT NOT NULL,
    beschreibung TEXT,
    fix_aufschlag NUMERIC,
    rundung_typ TEXT,
    mindestmarge_pct NUMERIC,
    auto_neuberechnung BOOLEAN,
    ist_default BOOLEAN,
    aktiv BOOLEAN,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE TABLE ErpPreisSpanne (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    schema_id INTEGER NOT NULL,
    preis_von NUMERIC NOT NULL,
    preis_bis NUMERIC,
    aufschlag_pct NUMERIC NOT NULL,
    sort_order INTEGER
);

CREATE TABLE ErpPreisGruppe (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    name TEXT NOT NULL,
    rabatt_pct NUMERIC,
    schema_id INTEGER,
    ist_default BOOLEAN,
    sort_order INTEGER,
    aktiv BOOLEAN,
    created_at TIMESTAMP
);

CREATE TABLE ErpLohnPeriode (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    jahr INTEGER NOT NULL,
    monat INTEGER NOT NULL,
    brutto_loehne NUMERIC,
    ag_sv_anteile NUMERIC,
    an_sv_anteile NUMERIC,
    lohnsteuer_gesamt NUMERIC,
    currency TEXT,
    notiz TEXT,
    gebucht_am TIMESTAMP,
    gebucht_von INTEGER,
    erfasst_am TIMESTAMP,
    erfasst_von INTEGER
);

CREATE TABLE DashboardDismissal (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    user_id INTEGER NOT NULL,
    item_key TEXT NOT NULL,
    created_at TIMESTAMP
);

CREATE TABLE CustomerRemoteDevice (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    provider TEXT NOT NULL,
    label TEXT,
    device_id TEXT NOT NULL,
    notizen TEXT,
    created_at TIMESTAMP
);

CREATE TABLE CrmCompany (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    rechtsform TEXT,
    branche TEXT,
    website TEXT,
    telefon TEXT,
    email TEXT,
    strasse TEXT,
    plz TEXT,
    ort TEXT,
    land TEXT,
    ust_id TEXT,
    customer_id INTEGER,
    inhaber_user_id INTEGER,
    notizen TEXT,
    tags TEXT,
    aktiv BOOLEAN,
    geloescht_am TIMESTAMP,
    erstellt_am TIMESTAMP,
    geaendert_am TIMESTAMP
);

CREATE TABLE CrmContact (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    company_id INTEGER,
    customer_id INTEGER,
    anrede TEXT,
    vorname TEXT,
    nachname TEXT NOT NULL,
    position TEXT,
    abteilung TEXT,
    email TEXT,
    telefon TEXT,
    mobil TEXT,
    ist_hauptkontakt BOOLEAN,
    quelle TEXT,
    notizen TEXT,
    tags TEXT,
    aktiv BOOLEAN,
    geloescht_am TIMESTAMP,
    erstellt_am TIMESTAMP,
    geaendert_am TIMESTAMP,
    carddav_uid TEXT,
    carddav_href TEXT,
    carddav_etag TEXT,
    carddav_synced_at TIMESTAMP
);

CREATE TABLE CrmPipeline (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    ist_default BOOLEAN,
    sort_order INTEGER,
    aktiv BOOLEAN,
    geloescht_am TIMESTAMP,
    erstellt_am TIMESTAMP
);

CREATE TABLE CrmStage (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    pipeline_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    sort_order INTEGER,
    wahrscheinlichkeit INTEGER,
    ist_gewonnen BOOLEAN,
    ist_verloren BOOLEAN,
    farbe TEXT,
    aktiv BOOLEAN,
    geloescht_am TIMESTAMP
);

CREATE TABLE CrmLead (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    titel TEXT NOT NULL,
    pipeline_id INTEGER NOT NULL,
    stage_id INTEGER NOT NULL,
    contact_id INTEGER,
    company_id INTEGER,
    customer_id INTEGER,
    wert_netto NUMERIC,
    waehrung TEXT,
    wahrscheinlichkeit INTEGER,
    quelle TEXT,
    zustaendig_user_id INTEGER,
    erwartetes_datum DATE,
    status TEXT,
    verloren_grund TEXT,
    source_cold_lead_id INTEGER,
    notizen TEXT,
    tags TEXT,
    category_id INTEGER,
    aktiv BOOLEAN,
    geloescht_am TIMESTAMP,
    erstellt_am TIMESTAMP,
    geaendert_am TIMESTAMP
);

CREATE TABLE CrmCategory (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    farbe TEXT,
    sort_order INTEGER,
    aktiv BOOLEAN,
    geloescht_am TIMESTAMP,
    erstellt_am TIMESTAMP
);

CREATE TABLE CrmActivity (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id INTEGER NOT NULL,
    typ TEXT NOT NULL,
    betreff TEXT,
    inhalt TEXT,
    richtung TEXT,
    ref_table TEXT,
    ref_id INTEGER,
    user_id INTEGER,
    zeitpunkt TIMESTAMP,
    geloescht_am TIMESTAMP,
    erstellt_am TIMESTAMP
);

CREATE TABLE CrmTask (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    titel TEXT NOT NULL,
    beschreibung TEXT,
    entity_type TEXT,
    entity_id INTEGER,
    faellig_am TIMESTAMP,
    erinnerung_am TIMESTAMP,
    prioritaet TEXT,
    status TEXT,
    zustaendig_user_id INTEGER,
    erledigt_am TIMESTAMP,
    geloescht_am TIMESTAMP,
    erstellt_am TIMESTAMP,
    geaendert_am TIMESTAMP
);

CREATE TABLE CrmDocument (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    titel TEXT NOT NULL,
    vorlage_id INTEGER,
    entity_type TEXT,
    entity_id INTEGER,
    customer_id INTEGER,
    storage_key TEXT,
    mime_type TEXT,
    format TEXT,
    version INTEGER,
    erp_document_id INTEGER,
    status TEXT,
    erstellt_von INTEGER,
    folder_id INTEGER,
    aktiv BOOLEAN,
    geloescht_am TIMESTAMP,
    erstellt_am TIMESTAMP,
    geaendert_am TIMESTAMP
);

CREATE TABLE CrmDocumentFolder (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id INTEGER NOT NULL,
    parent_id INTEGER,
    name TEXT NOT NULL,
    sort_order INTEGER,
    geloescht_am TIMESTAMP,
    erstellt_am TIMESTAMP
);

CREATE TABLE CrmDocTemplate (
    id INTEGER NOT NULL,
    tenant_id INTEGER,
    name TEXT NOT NULL,
    format TEXT,
    storage_key TEXT,
    beschreibung TEXT,
    kategorie TEXT,
    aktiv BOOLEAN,
    geloescht_am TIMESTAMP,
    erstellt_am TIMESTAMP
);

CREATE TABLE CrmDocumentLink (
    id INTEGER NOT NULL,
    tenant_id INTEGER NOT NULL,
    erp_document_id INTEGER NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id INTEGER NOT NULL,
    zugeordnet_von_user_id INTEGER,
    zugeordnet_am TIMESTAMP,
    quelle TEXT,
    geloescht_am TIMESTAMP,
    erstellt_am TIMESTAMP
);

CREATE TABLE RentedServerEvent (
    id INTEGER NOT NULL,
    ts TIMESTAMP NOT NULL,
    kind TEXT NOT NULL,
    level TEXT,
    message TEXT,
    payload TEXT
);

