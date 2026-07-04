"""@generated DO arm — faithful mirror of the `links` controller.
`osm.controllers.links.<is_a>(inp)` free functions (re-exported as
`osm.links`); standalone, not methods on the model. Call:
`osm.controllers.links.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def update_profile(inp: Input) -> Output:
    """`links:update_profile` — DO arm. Source: Profiles::LinksController#update_profile."""
    raise NotImplementedError("port Profiles::LinksController#update_profile")

