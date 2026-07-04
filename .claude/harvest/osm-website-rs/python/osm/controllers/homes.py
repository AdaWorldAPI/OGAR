"""@generated DO arm — faithful mirror of the `homes` controller.
`osm.controllers.homes.<is_a>(inp)` free functions (re-exported as
`osm.homes`); standalone, not methods on the model. Call:
`osm.controllers.homes.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`homes:show` — DO arm. Source: Accounts::HomesController#show."""
    raise NotImplementedError("port Accounts::HomesController#show")

