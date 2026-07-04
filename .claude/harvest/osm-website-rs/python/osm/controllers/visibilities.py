"""@generated DO arm — faithful mirror of the `visibilities` controller.
`osm.controllers.visibilities.<is_a>(inp)` free functions (re-exported as
`osm.visibilities`); standalone, not methods on the model. Call:
`osm.controllers.visibilities.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`visibilities:create` — DO arm. Source: Api::ChangesetComments::VisibilitiesController#create."""
    raise NotImplementedError("port Api::ChangesetComments::VisibilitiesController#create")

def delete(inp: Input) -> Output:
    """`visibilities:delete` — DO arm. Source: Api::ChangesetComments::VisibilitiesController#destroy."""
    raise NotImplementedError("port Api::ChangesetComments::VisibilitiesController#destroy")

