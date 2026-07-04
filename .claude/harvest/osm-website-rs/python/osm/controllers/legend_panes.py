"""@generated DO arm — faithful mirror of the `legend_panes` controller.
`osm.controllers.legend_panes.<is_a>(inp)` free functions (re-exported as
`osm.legend_panes`); standalone, not methods on the model. Call:
`osm.controllers.legend_panes.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`legend_panes:show` — DO arm. Source: LegendPanesController#show."""
    raise NotImplementedError("port LegendPanesController#show")

