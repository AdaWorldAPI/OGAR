"""@generated DO arm — faithful mirror of the `maps` controller.
`osm.controllers.maps.<is_a>(inp)` free functions (re-exported as
`osm.maps`); standalone, not methods on the model. Call:
`osm.controllers.maps.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`maps:show` — DO arm. Source: Api::MapsController#show."""
    raise NotImplementedError("port Api::MapsController#show")

