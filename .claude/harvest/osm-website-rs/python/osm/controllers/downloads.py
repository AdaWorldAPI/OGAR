"""@generated DO arm — faithful mirror of the `downloads` controller.
`osm.controllers.downloads.<is_a>(inp)` free functions (re-exported as
`osm.downloads`); standalone, not methods on the model. Call:
`osm.controllers.downloads.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`downloads:show` — DO arm. Source: Api::Changesets::DownloadsController#show."""
    raise NotImplementedError("port Api::Changesets::DownloadsController#show")

def show_redactions(inp: Input) -> Output:
    """`downloads:show_redactions?` — DO arm. Source: Api::Changesets::DownloadsController#show_redactions?."""
    raise NotImplementedError("port Api::Changesets::DownloadsController#show_redactions?")

