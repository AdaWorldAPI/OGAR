"""@generated DO arm — faithful mirror of the `permissions` controller.
`osm.controllers.permissions.<is_a>(inp)` free functions (re-exported as
`osm.permissions`); standalone, not methods on the model. Call:
`osm.controllers.permissions.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`permissions:show` — DO arm. Source: Api::PermissionsController#show."""
    raise NotImplementedError("port Api::PermissionsController#show")

