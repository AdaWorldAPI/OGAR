"""@generated DO arm — faithful mirror of the `layers_panes` controller.
`osm.controllers.layers_panes.<is_a>(inp)` free functions (re-exported as
`osm.layers_panes`); standalone, not methods on the model. Call:
`osm.controllers.layers_panes.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`layers_panes:show` — DO arm. Source: LayersPanesController#show."""
    raise NotImplementedError("port LayersPanesController#show")

