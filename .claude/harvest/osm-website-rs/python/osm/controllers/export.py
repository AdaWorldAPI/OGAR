"""@generated DO arm — faithful mirror of the `export` controller.
`osm.controllers.export.<is_a>(inp)` free functions (re-exported as
`osm.export`); standalone, not methods on the model. Call:
`osm.controllers.export.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`export:create` — DO arm. Source: ExportController#create."""
    raise NotImplementedError("port ExportController#create")

def show(inp: Input) -> Output:
    """`export:show` — DO arm. Source: ExportController#show."""
    raise NotImplementedError("port ExportController#show")

