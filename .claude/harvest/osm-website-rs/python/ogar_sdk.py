"""ogar_sdk — thin Python client over the OGAR codebook substrate.

The Python mirror of Rust `lance_graph_contract::ogar_codebook`: pull a concept's
stable classid, or compose a render-classid with an app prefix. Domain-agnostic —
OSM (0x0F) and odoo/commerce (0x02) pull identically.

    >>> from ogar_sdk import class_id, render_classid
    >>> class_id("osm_node")
    3841
    >>> hex(render_classid("account_move", 0x0002))   # odoo, canon-high
    '0x2020002'
"""
from typing import Optional

# The pulled substrate — a subset of ogar-vocab's CODEBOOK (concept -> u16 id).
CODEBOOK = {
    # 0x0F — Geo (OpenStreetMap)
    "osm_node": 0x0F01, "osm_way": 0x0F02, "osm_relation": 0x0F03,
    "osm_changeset": 0x0F04, "osm_element_tag": 0x0F05, "osm_relation_member": 0x0F06,
    "osm_way_node": 0x0F07, "osm_note": 0x0F08, "osm_gpx_trace": 0x0F09, "osm_user": 0x0F0A,
    # 0x02 — Commerce (odoo / ERP) — same pull, different domain
    "commercial_document": 0x0202, "product": 0x0207, "accounting_account": 0x0208,
}
# Convenience alias used in the odoo docstring example.
CODEBOOK["account_move"] = CODEBOOK["commercial_document"]


def class_id(concept: str) -> Optional[int]:
    """Resolve a canonical-concept string to its stable u16 codebook id."""
    return CODEBOOK.get(concept)


def render_classid(concept: str, app_prefix: int) -> Optional[int]:
    """Compose a 32-bit render classid: canon (concept) HIGH, custom (prefix) LOW.

    Mirrors the post-2026-07-02 canon-high flip: `(concept << 16) | prefix`.
    """
    cid = class_id(concept)
    if cid is None:
        return None
    return (cid << 16) | (app_prefix & 0xFFFF)
