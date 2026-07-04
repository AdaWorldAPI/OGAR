"""@generated DO arm — faithful mirror of the `share_panes` controller.
`osm.controllers.share_panes.<is_a>(inp)` free functions (re-exported as
`osm.share_panes`); standalone, not methods on the model. Call:
`osm.controllers.share_panes.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`share_panes:show` — DO arm. Source: SharePanesController#show."""
    raise NotImplementedError("port SharePanesController#show")

