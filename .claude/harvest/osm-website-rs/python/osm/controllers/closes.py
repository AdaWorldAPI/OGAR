"""@generated DO arm — faithful mirror of the `closes` controller.
`osm.controllers.closes.<is_a>(inp)` free functions (re-exported as
`osm.closes`); standalone, not methods on the model. Call:
`osm.controllers.closes.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def update(inp: Input) -> Output:
    """`closes:update` — DO arm. Source: Api::Changesets::ClosesController#update."""
    raise NotImplementedError("port Api::Changesets::ClosesController#update")

