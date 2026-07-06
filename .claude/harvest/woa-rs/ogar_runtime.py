"""ogar_runtime — reference wrapper-contract for emit_python output.

The Python mirror of the Rust consumer's wrapper types (OgScalar / ToOne /
ToMany, lance-graph-contract shape). A real consumer supplies its own
ogar_runtime on sys.path; this is the shipped reference so emitted modules
py_compile + import with no setup.
Dataclass annotations are not runtime-enforced, so these permissive aliases
suffice.
"""
from __future__ import annotations
from typing import Generic, TypeVar

_T = TypeVar("_T")


class ToOne(Generic[_T]):
    """To-one relation wrapper (forward-ref comodel)."""


class ToMany(Generic[_T]):
    """To-many relation wrapper (forward-ref comodel)."""


OgScalar = object
OgStr = str
OgInt = int
OgFloat = float
OgMoney = object
OgBool = bool
OgDate = object
OgDateTime = object
OgBytes = bytes
OgSelection = object
OgJson = object
