"""@generated DO arm — faithful mirror of the `tracepoints` controller.
`osm.controllers.tracepoints.<is_a>(inp)` free functions (re-exported as
`osm.tracepoints`); standalone, not methods on the model. Call:
`osm.controllers.tracepoints.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def list(inp: Input) -> Output:
    """`tracepoints:list` — DO arm. Source: Api::TracepointsController#index."""
    raise NotImplementedError("port Api::TracepointsController#index")

