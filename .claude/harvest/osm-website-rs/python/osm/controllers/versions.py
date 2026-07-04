"""@generated DO arm — faithful mirror of the `versions` controller.
`osm.controllers.versions.<is_a>(inp)` free functions (re-exported as
`osm.versions`); standalone, not methods on the model. Call:
`osm.controllers.versions.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`versions:show` — DO arm. Source: Api::VersionsController#show."""
    raise NotImplementedError("port Api::VersionsController#show")

