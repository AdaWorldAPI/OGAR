"""@generated DO arm — faithful mirror of the `old_relations` controller.
`osm.controllers.old_relations.<is_a>(inp)` free functions (re-exported as
`osm.old_relations`); standalone, not methods on the model. Call:
`osm.controllers.old_relations.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def list(inp: Input) -> Output:
    """`old_relations:list` — DO arm. Source: OldRelationsController#index."""
    raise NotImplementedError("port OldRelationsController#index")

def lookup_old_element(inp: Input) -> Output:
    """`old_relations:lookup_old_element` — DO arm. Source: Api::OldRelationsController#lookup_old_element."""
    raise NotImplementedError("port Api::OldRelationsController#lookup_old_element")

def lookup_old_element_versions(inp: Input) -> Output:
    """`old_relations:lookup_old_element_versions` — DO arm. Source: Api::OldRelationsController#lookup_old_element_versions."""
    raise NotImplementedError("port Api::OldRelationsController#lookup_old_element_versions")

def show(inp: Input) -> Output:
    """`old_relations:show` — DO arm. Source: OldRelationsController#show."""
    raise NotImplementedError("port OldRelationsController#show")

