"""@generated DO arm — faithful mirror of the `uploads` controller.
`osm.controllers.uploads.<is_a>(inp)` free functions (re-exported as
`osm.uploads`); standalone, not methods on the model. Call:
`osm.controllers.uploads.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`uploads:create` — DO arm. Source: Api::Changesets::UploadsController#create."""
    raise NotImplementedError("port Api::Changesets::UploadsController#create")

