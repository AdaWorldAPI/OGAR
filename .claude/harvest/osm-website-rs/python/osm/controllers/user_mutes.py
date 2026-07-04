"""@generated DO arm — faithful mirror of the `user_mutes` controller.
`osm.controllers.user_mutes.<is_a>(inp)` free functions (re-exported as
`osm.user_mutes`); standalone, not methods on the model. Call:
`osm.controllers.user_mutes.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`user_mutes:create` — DO arm. Source: UserMutesController#create."""
    raise NotImplementedError("port UserMutesController#create")

def delete(inp: Input) -> Output:
    """`user_mutes:delete` — DO arm. Source: UserMutesController#destroy."""
    raise NotImplementedError("port UserMutesController#destroy")

def list(inp: Input) -> Output:
    """`user_mutes:list` — DO arm. Source: UserMutesController#index."""
    raise NotImplementedError("port UserMutesController#index")

