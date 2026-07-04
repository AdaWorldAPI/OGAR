"""@generated DO arm — faithful mirror of the `feeds` controller.
`osm.controllers.feeds.<is_a>(inp)` free functions (re-exported as
`osm.feeds`); standalone, not methods on the model. Call:
`osm.controllers.feeds.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`feeds:show` — DO arm. Sources (canonical tile): ChangesetComments::FeedsController#show, Traces::FeedsController#show."""
    raise NotImplementedError("port ChangesetComments::FeedsController#show")

