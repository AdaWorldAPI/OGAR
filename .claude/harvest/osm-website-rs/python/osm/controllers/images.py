"""@generated DO arm — faithful mirror of the `images` controller.
`osm.controllers.images.<is_a>(inp)` free functions (re-exported as
`osm.images`); standalone, not methods on the model. Call:
`osm.controllers.images.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def update_profile(inp: Input) -> Output:
    """`images:update_profile` — DO arm. Source: Profiles::ImagesController#update_profile."""
    raise NotImplementedError("port Profiles::ImagesController#update_profile")

