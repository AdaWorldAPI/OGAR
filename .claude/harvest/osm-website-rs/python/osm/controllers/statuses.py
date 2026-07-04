"""@generated DO arm — faithful mirror of the `statuses` controller.
`osm.controllers.statuses.<is_a>(inp)` free functions (re-exported as
`osm.statuses`); standalone, not methods on the model. Call:
`osm.controllers.statuses.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def lookup_user_by_name(inp: Input) -> Output:
    """`statuses:lookup_user_by_name` — DO arm. Source: Users::StatusesController#lookup_user_by_name."""
    raise NotImplementedError("port Users::StatusesController#lookup_user_by_name")

def update(inp: Input) -> Output:
    """`statuses:update` — DO arm. Source: Users::StatusesController#update."""
    raise NotImplementedError("port Users::StatusesController#update")

