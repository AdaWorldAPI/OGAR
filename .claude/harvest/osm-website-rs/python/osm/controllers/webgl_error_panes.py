"""@generated DO arm — faithful mirror of the `webgl_error_panes` controller.
`osm.controllers.webgl_error_panes.<is_a>(inp)` free functions (re-exported as
`osm.webgl_error_panes`); standalone, not methods on the model. Call:
`osm.controllers.webgl_error_panes.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`webgl_error_panes:show` — DO arm. Source: WebglErrorPanesController#show."""
    raise NotImplementedError("port WebglErrorPanesController#show")

