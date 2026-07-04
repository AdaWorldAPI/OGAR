"""@generated DO arm — faithful mirror of the `accounts` controller.
`osm.controllers.accounts.<is_a>(inp)` free functions (re-exported as
`osm.accounts`); standalone, not methods on the model. Call:
`osm.controllers.accounts.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def delete(inp: Input) -> Output:
    """`accounts:delete` — DO arm. Source: AccountsController#destroy."""
    raise NotImplementedError("port AccountsController#destroy")

def show(inp: Input) -> Output:
    """`accounts:show` — DO arm. Source: AccountsController#show."""
    raise NotImplementedError("port AccountsController#show")

def update(inp: Input) -> Output:
    """`accounts:update` — DO arm. Source: AccountsController#update."""
    raise NotImplementedError("port AccountsController#update")

