"""@generated DO arm — faithful mirror of the `old_nodes` controller.
`osm.controllers.old_nodes.<is_a>(inp)` free functions (re-exported as
`osm.old_nodes`); standalone, not methods on the model. Call:
`osm.controllers.old_nodes.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def list(inp: Input) -> Output:
    """`old_nodes:list` — DO arm. Source: OldNodesController#index."""
    raise NotImplementedError("port OldNodesController#index")

def lookup_old_element(inp: Input) -> Output:
    """`old_nodes:lookup_old_element` — DO arm. Source: Api::OldNodesController#lookup_old_element."""
    raise NotImplementedError("port Api::OldNodesController#lookup_old_element")

def lookup_old_element_versions(inp: Input) -> Output:
    """`old_nodes:lookup_old_element_versions` — DO arm. Source: Api::OldNodesController#lookup_old_element_versions."""
    raise NotImplementedError("port Api::OldNodesController#lookup_old_element_versions")

def show(inp: Input) -> Output:
    """`old_nodes:show` — DO arm. Source: OldNodesController#show."""
    raise NotImplementedError("port OldNodesController#show")

