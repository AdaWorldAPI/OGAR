"""@generated DO arm — faithful mirror of the `old_elements` controller.
`osm.controllers.old_elements.<is_a>(inp)` free functions (re-exported as
`osm.old_elements`); standalone, not methods on the model. Call:
`osm.controllers.old_elements.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def list(inp: Input) -> Output:
    """`old_elements:list` — DO arm. Source: Api::OldElementsController#index."""
    raise NotImplementedError("port Api::OldElementsController#index")

def require_moderator_for_unredacted_history(inp: Input) -> Output:
    """`old_elements:require_moderator_for_unredacted_history` — DO arm. Source: OldElementsController#require_moderator_for_unredacted_history."""
    raise NotImplementedError("port OldElementsController#require_moderator_for_unredacted_history")

def show(inp: Input) -> Output:
    """`old_elements:show` — DO arm. Source: Api::OldElementsController#show."""
    raise NotImplementedError("port Api::OldElementsController#show")

def show_redactions(inp: Input) -> Output:
    """`old_elements:show_redactions?` — DO arm. Source: Api::OldElementsController#show_redactions?."""
    raise NotImplementedError("port Api::OldElementsController#show_redactions?")

