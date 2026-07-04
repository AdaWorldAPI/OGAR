"""@generated DO arm — faithful mirror of the `muted_inboxes` controller.
`osm.controllers.muted_inboxes.<is_a>(inp)` free functions (re-exported as
`osm.muted_inboxes`); standalone, not methods on the model. Call:
`osm.controllers.muted_inboxes.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`muted_inboxes:show` — DO arm. Source: Messages::MutedInboxesController#show."""
    raise NotImplementedError("port Messages::MutedInboxesController#show")

