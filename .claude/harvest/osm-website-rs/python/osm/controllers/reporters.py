"""@generated DO arm — faithful mirror of the `reporters` controller.
`osm.controllers.reporters.<is_a>(inp)` free functions (re-exported as
`osm.reporters`); standalone, not methods on the model. Call:
`osm.controllers.reporters.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def list(inp: Input) -> Output:
    """`reporters:list` — DO arm. Source: Issues::ReportersController#index."""
    raise NotImplementedError("port Issues::ReportersController#index")

