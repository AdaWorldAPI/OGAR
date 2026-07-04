"""@generated DO arm — faithful mirror of the `active_lists` controller.
`osm.controllers.active_lists.<is_a>(inp)` free functions (re-exported as
`osm.active_lists`); standalone, not methods on the model. Call:
`osm.controllers.active_lists.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`active_lists:show` — DO arm. Source: Api::UserBlocks::ActiveListsController#show."""
    raise NotImplementedError("port Api::UserBlocks::ActiveListsController#show")

