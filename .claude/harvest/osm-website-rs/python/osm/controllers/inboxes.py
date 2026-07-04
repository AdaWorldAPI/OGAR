"""@generated DO arm — faithful mirror of the `inboxes` controller.
`osm.controllers.inboxes.<is_a>(inp)` free functions (re-exported as
`osm.inboxes`); standalone, not methods on the model. Call:
`osm.controllers.inboxes.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`inboxes:show` — DO arm. Sources (canonical tile): Api::Messages::InboxesController#show, Messages::InboxesController#show."""
    raise NotImplementedError("port Api::Messages::InboxesController#show")

