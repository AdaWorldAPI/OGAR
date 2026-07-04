"""@generated DO arm — faithful mirror of the `follows` controller.
`osm.controllers.follows.<is_a>(inp)` free functions (re-exported as
`osm.follows`); standalone, not methods on the model. Call:
`osm.controllers.follows.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`follows:create` — DO arm. Source: FollowsController#create."""
    raise NotImplementedError("port FollowsController#create")

def delete(inp: Input) -> Output:
    """`follows:delete` — DO arm. Source: FollowsController#destroy."""
    raise NotImplementedError("port FollowsController#destroy")

def show(inp: Input) -> Output:
    """`follows:show` — DO arm. Source: FollowsController#show."""
    raise NotImplementedError("port FollowsController#show")

