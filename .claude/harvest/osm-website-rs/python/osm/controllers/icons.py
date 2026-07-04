"""@generated DO arm — faithful mirror of the `icons` controller.
`osm.controllers.icons.<is_a>(inp)` free functions (re-exported as
`osm.icons`); standalone, not methods on the model. Call:
`osm.controllers.icons.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`icons:show` — DO arm. Source: Traces::IconsController#show."""
    raise NotImplementedError("port Traces::IconsController#show")

