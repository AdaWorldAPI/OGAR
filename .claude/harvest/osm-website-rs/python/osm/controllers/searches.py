"""@generated DO arm — faithful mirror of the `searches` controller.
`osm.controllers.searches.<is_a>(inp)` free functions (re-exported as
`osm.searches`); standalone, not methods on the model. Call:
`osm.controllers.searches.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def dms_regexp(inp: Input) -> Output:
    """`searches:dms_regexp` — DO arm. Source: SearchesController#dms_regexp."""
    raise NotImplementedError("port SearchesController#dms_regexp")

def normalize_params(inp: Input) -> Output:
    """`searches:normalize_params` — DO arm. Source: SearchesController#normalize_params."""
    raise NotImplementedError("port SearchesController#normalize_params")

def show(inp: Input) -> Output:
    """`searches:show` — DO arm. Source: SearchesController#show."""
    raise NotImplementedError("port SearchesController#show")

def to_decdeg(inp: Input) -> Output:
    """`searches:to_decdeg` — DO arm. Source: SearchesController#to_decdeg."""
    raise NotImplementedError("port SearchesController#to_decdeg")

