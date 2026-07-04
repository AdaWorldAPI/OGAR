"""@generated DO arm — faithful mirror of the `capabilities` controller.
`osm.controllers.capabilities.<is_a>(inp)` free functions (re-exported as
`osm.capabilities`); standalone, not methods on the model. Call:
`osm.controllers.capabilities.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`capabilities:show` — DO arm. Source: Api::CapabilitiesController#show."""
    raise NotImplementedError("port Api::CapabilitiesController#show")

