"""@generated DO arm — faithful mirror of the `old_ways` controller.
`osm.controllers.old_ways.<is_a>(inp)` free functions (re-exported as
`osm.old_ways`); standalone, not methods on the model. Call:
`osm.controllers.old_ways.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def list(inp: Input) -> Output:
    """`old_ways:list` — DO arm. Source: OldWaysController#index."""
    raise NotImplementedError("port OldWaysController#index")

def lookup_old_element(inp: Input) -> Output:
    """`old_ways:lookup_old_element` — DO arm. Source: Api::OldWaysController#lookup_old_element."""
    raise NotImplementedError("port Api::OldWaysController#lookup_old_element")

def lookup_old_element_versions(inp: Input) -> Output:
    """`old_ways:lookup_old_element_versions` — DO arm. Source: Api::OldWaysController#lookup_old_element_versions."""
    raise NotImplementedError("port Api::OldWaysController#lookup_old_element_versions")

def show(inp: Input) -> Output:
    """`old_ways:show` — DO arm. Source: OldWaysController#show."""
    raise NotImplementedError("port OldWaysController#show")

