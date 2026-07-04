"""@generated DO arm — faithful mirror of the `outboxes` controller.
`osm.controllers.outboxes.<is_a>(inp)` free functions (re-exported as
`osm.outboxes`); standalone, not methods on the model. Call:
`osm.controllers.outboxes.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`outboxes:show` — DO arm. Sources (canonical tile): Api::Messages::OutboxesController#show, Messages::OutboxesController#show."""
    raise NotImplementedError("port Api::Messages::OutboxesController#show")

