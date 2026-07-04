"""@generated DO arm — faithful mirror of the `lists` controller.
`osm.controllers.lists.<is_a>(inp)` free functions (re-exported as
`osm.lists`); standalone, not methods on the model. Call:
`osm.controllers.lists.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`lists:show` — DO arm. Source: Users::ListsController#show."""
    raise NotImplementedError("port Users::ListsController#show")

def update(inp: Input) -> Output:
    """`lists:update` — DO arm. Source: Users::ListsController#update."""
    raise NotImplementedError("port Users::ListsController#update")

