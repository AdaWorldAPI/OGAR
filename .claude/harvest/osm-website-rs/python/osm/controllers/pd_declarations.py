"""@generated DO arm — faithful mirror of the `pd_declarations` controller.
`osm.controllers.pd_declarations.<is_a>(inp)` free functions (re-exported as
`osm.pd_declarations`); standalone, not methods on the model. Call:
`osm.controllers.pd_declarations.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`pd_declarations:create` — DO arm. Source: Accounts::PdDeclarationsController#create."""
    raise NotImplementedError("port Accounts::PdDeclarationsController#create")

def show(inp: Input) -> Output:
    """`pd_declarations:show` — DO arm. Source: Accounts::PdDeclarationsController#show."""
    raise NotImplementedError("port Accounts::PdDeclarationsController#show")

