"""@generated DO arm — faithful mirror of the `directions` controller.
`osm.controllers.directions.<is_a>(inp)` free functions (re-exported as
`osm.directions`); standalone, not methods on the model. Call:
`osm.controllers.directions.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`directions:show` — DO arm. Source: DirectionsController#show."""
    raise NotImplementedError("port DirectionsController#show")

