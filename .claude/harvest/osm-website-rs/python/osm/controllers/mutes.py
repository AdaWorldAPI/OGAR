"""@generated DO arm — faithful mirror of the `mutes` controller.
`osm.controllers.mutes.<is_a>(inp)` free functions (re-exported as
`osm.mutes`); standalone, not methods on the model. Call:
`osm.controllers.mutes.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def delete(inp: Input) -> Output:
    """`mutes:delete` — DO arm. Source: Messages::MutesController#destroy."""
    raise NotImplementedError("port Messages::MutesController#destroy")

