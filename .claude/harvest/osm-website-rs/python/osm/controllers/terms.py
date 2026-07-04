"""@generated DO arm — faithful mirror of the `terms` controller.
`osm.controllers.terms.<is_a>(inp)` free functions (re-exported as
`osm.terms`); standalone, not methods on the model. Call:
`osm.controllers.terms.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`terms:show` — DO arm. Source: Accounts::TermsController#show."""
    raise NotImplementedError("port Accounts::TermsController#show")

def update(inp: Input) -> Output:
    """`terms:update` — DO arm. Source: Accounts::TermsController#update."""
    raise NotImplementedError("port Accounts::TermsController#update")

