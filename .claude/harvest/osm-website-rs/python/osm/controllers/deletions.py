"""@generated DO arm — faithful mirror of the `deletions` controller.
`osm.controllers.deletions.<is_a>(inp)` free functions (re-exported as
`osm.deletions`); standalone, not methods on the model. Call:
`osm.controllers.deletions.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`deletions:show` — DO arm. Source: Accounts::DeletionsController#show."""
    raise NotImplementedError("port Accounts::DeletionsController#show")

